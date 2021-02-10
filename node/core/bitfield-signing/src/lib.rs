// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! The bitfield signing subsystem produces `SignedAvailabilityBitfield`s once per block.

#![deny(unused_crate_dependencies)]
#![warn(missing_docs)]
#![recursion_limit = "256"]

use futures::{
    channel::{mpsc, oneshot},
    future,
    lock::Mutex,
    prelude::*,
    Future,
};
use indracore_node_subsystem::{
    errors::RuntimeApiError,
    messages::{
        AllMessages, AvailabilityStoreMessage, BitfieldDistributionMessage, BitfieldSigningMessage,
        RuntimeApiMessage, RuntimeApiRequest,
    },
};
use indracore_node_subsystem_util::{
    self as util,
    metrics::{self, prometheus},
    FromJobCommand, JobManager, JobTrait, Validator,
};
use indracore_primitives::v1::{AvailabilityBitfield, CoreState, Hash, ValidatorIndex};
use sp_keystore::{Error as KeystoreError, SyncCryptoStorePtr};
use std::{iter::FromIterator, pin::Pin, time::Duration};
use thiserror::Error;
use tracing_futures as _;
use wasm_timer::{Delay, Instant};

/// Delay between starting a bitfield signing job and its attempting to create a bitfield.
const JOB_DELAY: Duration = Duration::from_millis(1500);
const LOG_TARGET: &str = "bitfield_signing";

/// Each `BitfieldSigningJob` prepares a signed bitfield for a single relay parent.
pub struct BitfieldSigningJob;

/// Errors we may encounter in the course of executing the `BitfieldSigningSubsystem`.
#[derive(Debug, Error)]
pub enum Error {
    /// error propagated from the utility subsystem
    #[error(transparent)]
    Util(#[from] util::Error),
    /// io error
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// a one shot channel was canceled
    #[error(transparent)]
    Oneshot(#[from] oneshot::Canceled),
    /// a mspc channel failed to send
    #[error(transparent)]
    MpscSend(#[from] mpsc::SendError),
    /// the runtime API failed to return what we wanted
    #[error(transparent)]
    Runtime(#[from] RuntimeApiError),
    /// the keystore failed to process signing request
    #[error("Keystore failed: {0:?}")]
    Keystore(KeystoreError),
}

/// If there is a candidate pending availability, query the Availability Store
/// for whether we have the availability chunk for our validator index.
#[tracing::instrument(level = "trace", skip(sender), fields(subsystem = LOG_TARGET))]
async fn get_core_availability(
    relay_parent: Hash,
    core: CoreState,
    validator_idx: ValidatorIndex,
    sender: &Mutex<&mut mpsc::Sender<FromJobCommand>>,
) -> Result<bool, Error> {
    if let CoreState::Occupied(core) = core {
        let (tx, rx) = oneshot::channel();
        sender
            .lock()
            .await
            .send(
                AllMessages::from(RuntimeApiMessage::Request(
                    relay_parent,
                    RuntimeApiRequest::CandidatePendingAvailability(core.para_id, tx),
                ))
                .into(),
            )
            .await?;

        let committed_candidate_receipt = match rx.await? {
            Ok(Some(ccr)) => ccr,
            Ok(None) => return Ok(false),
            Err(e) => {
                // Don't take down the node on runtime API errors.
                tracing::warn!(target: LOG_TARGET, err = ?e, "Encountered a runtime API error");
                return Ok(false);
            }
        };
        let (tx, rx) = oneshot::channel();
        sender
            .lock()
            .await
            .send(
                AllMessages::from(AvailabilityStoreMessage::QueryChunkAvailability(
                    committed_candidate_receipt.hash(),
                    validator_idx,
                    tx,
                ))
                .into(),
            )
            .await?;
        return rx.await.map_err(Into::into);
    }

    Ok(false)
}

/// delegates to the v1 runtime API
async fn get_availability_cores(
    relay_parent: Hash,
    sender: &mut mpsc::Sender<FromJobCommand>,
) -> Result<Vec<CoreState>, Error> {
    let (tx, rx) = oneshot::channel();
    sender
        .send(
            AllMessages::from(RuntimeApiMessage::Request(
                relay_parent,
                RuntimeApiRequest::AvailabilityCores(tx),
            ))
            .into(),
        )
        .await?;
    match rx.await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(runtime_err)) => Err(runtime_err.into()),
        Err(err) => Err(err.into()),
    }
}

/// - get the list of core states from the runtime
/// - for each core, concurrently determine chunk availability (see `get_core_availability`)
/// - return the bitfield if there were no errors at any point in this process
///   (otherwise, it's prone to false negatives)
#[tracing::instrument(level = "trace", skip(sender), fields(subsystem = LOG_TARGET))]
async fn construct_availability_bitfield(
    relay_parent: Hash,
    validator_idx: ValidatorIndex,
    sender: &mut mpsc::Sender<FromJobCommand>,
) -> Result<AvailabilityBitfield, Error> {
    // get the set of availability cores from the runtime
    let availability_cores = get_availability_cores(relay_parent, sender).await?;

    // Wrap the sender in a Mutex to share it between the futures.
    //
    // We use a `Mutex` here to not `clone` the sender inside the future, because
    // cloning the sender will always increase the capacity of the channel by one.
    // (for the lifetime of the sender)
    let sender = Mutex::new(sender);

    // Handle all cores concurrently
    // `try_join_all` returns all results in the same order as the input futures.
    let results = future::try_join_all(
        availability_cores
            .into_iter()
            .map(|core| get_core_availability(relay_parent, core, validator_idx, &sender)),
    )
    .await?;

    Ok(AvailabilityBitfield(FromIterator::from_iter(results)))
}

#[derive(Clone)]
struct MetricsInner {
    bitfields_signed_total: prometheus::Counter<prometheus::U64>,
    run: prometheus::Histogram,
}

/// Bitfield signing metrics.
#[derive(Default, Clone)]
pub struct Metrics(Option<MetricsInner>);

impl Metrics {
    fn on_bitfield_signed(&self) {
        if let Some(metrics) = &self.0 {
            metrics.bitfields_signed_total.inc();
        }
    }

    /// Provide a timer for `prune_povs` which observes on drop.
    fn time_run(&self) -> Option<metrics::prometheus::prometheus::HistogramTimer> {
        self.0.as_ref().map(|metrics| metrics.run.start_timer())
    }
}

impl metrics::Metrics for Metrics {
    fn try_register(registry: &prometheus::Registry) -> Result<Self, prometheus::PrometheusError> {
        let metrics = MetricsInner {
            bitfields_signed_total: prometheus::register(
                prometheus::Counter::new(
                    "parachain_bitfields_signed_total",
                    "Number of bitfields signed.",
                )?,
                registry,
            )?,
            run: prometheus::register(
                prometheus::Histogram::with_opts(prometheus::HistogramOpts::new(
                    "parachain_bitfield_signing_run",
                    "Time spent within `bitfield_signing::run`",
                ))?,
                registry,
            )?,
        };
        Ok(Metrics(Some(metrics)))
    }
}

impl JobTrait for BitfieldSigningJob {
    type ToJob = BitfieldSigningMessage;
    type Error = Error;
    type RunArgs = SyncCryptoStorePtr;
    type Metrics = Metrics;

    const NAME: &'static str = "BitfieldSigningJob";

    /// Run a job for the parent block indicated
    #[tracing::instrument(skip(keystore, metrics, _receiver, sender), fields(subsystem = LOG_TARGET))]
    fn run(
        relay_parent: Hash,
        keystore: Self::RunArgs,
        metrics: Self::Metrics,
        _receiver: mpsc::Receiver<BitfieldSigningMessage>,
        mut sender: mpsc::Sender<FromJobCommand>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>> {
        let metrics = metrics;
        async move {
			let wait_until = Instant::now() + JOB_DELAY;

			// now do all the work we can before we need to wait for the availability store
			// if we're not a validator, we can just succeed effortlessly
			let validator = match Validator::new(relay_parent, keystore.clone(), sender.clone()).await {
				Ok(validator) => validator,
				Err(util::Error::NotAValidator) => return Ok(()),
				Err(err) => return Err(Error::Util(err)),
			};

			// wait a bit before doing anything else
			Delay::new_at(wait_until).await?;

			// this timer does not appear at the head of the function because we don't want to include
			// JOB_DELAY each time.
			let _timer = metrics.time_run();

			let bitfield =
				match construct_availability_bitfield(relay_parent, validator.index(), &mut sender).await
			{
				Err(Error::Runtime(runtime_err)) => {
					// Don't take down the node on runtime API errors.
					tracing::warn!(target: LOG_TARGET, err = ?runtime_err, "Encountered a runtime API error");
					return Ok(());
				}
				Err(err) => return Err(err),
				Ok(bitfield) => bitfield,
			};

			let signed_bitfield = validator
				.sign(keystore.clone(), bitfield)
				.await
				.map_err(Error::Keystore)?;
			metrics.on_bitfield_signed();

			sender
				.send(
					AllMessages::from(
						BitfieldDistributionMessage::DistributeBitfield(relay_parent, signed_bitfield),
					).into(),
				)
				.await
				.map_err(Into::into)
		}
		.boxed()
    }
}

/// BitfieldSigningSubsystem manages a number of bitfield signing jobs.
pub type BitfieldSigningSubsystem<Spawner, Context> =
    JobManager<Spawner, Context, BitfieldSigningJob>;
