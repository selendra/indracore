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

//! Utility function to make it easier to connect to validators.

use std::collections::HashMap;
use std::pin::Pin;

use futures::{
    channel::mpsc,
    stream,
    task::{self, Poll},
    StreamExt,
};
use streamunordered::{StreamUnordered, StreamYield};

use crate::Error;
use indracore_node_subsystem::{
    errors::RuntimeApiError,
    messages::{AllMessages, NetworkBridgeMessage},
    SubsystemContext,
};
use indracore_primitives::v1::{AuthorityDiscoveryId, Hash, SessionIndex, ValidatorId};
use sc_network::PeerId;

/// Utility function to make it easier to connect to validators.
pub async fn connect_to_validators<Context: SubsystemContext>(
    ctx: &mut Context,
    relay_parent: Hash,
    validators: Vec<ValidatorId>,
) -> Result<ConnectionRequest, Error> {
    let current_index = crate::request_session_index_for_child_ctx(relay_parent, ctx)
        .await?
        .await??;
    connect_to_past_session_validators(ctx, relay_parent, validators, current_index).await
}

/// Utility function to make it easier to connect to validators in the past sessions.
pub async fn connect_to_past_session_validators<Context: SubsystemContext>(
    ctx: &mut Context,
    relay_parent: Hash,
    validators: Vec<ValidatorId>,
    session_index: SessionIndex,
) -> Result<ConnectionRequest, Error> {
    let session_info = crate::request_session_info_ctx(relay_parent, session_index, ctx)
        .await?
        .await??;

    let (session_validators, discovery_keys) = match session_info {
        Some(info) => (info.validators, info.discovery_keys),
        None => {
            return Err(RuntimeApiError::from(format!(
                "No SessionInfo found for the index {}",
                session_index
            ))
            .into())
        }
    };

    let id_to_index = session_validators
        .iter()
        .zip(0usize..)
        .collect::<HashMap<_, _>>();

    // We assume the same ordering in authorities as in validators so we can do an index search
    let maybe_authorities: Vec<_> = validators
        .iter()
        .map(|id| {
            let validator_index = id_to_index.get(&id);
            validator_index.and_then(|i| discovery_keys.get(*i).cloned())
        })
        .collect();

    let authorities: Vec<_> = maybe_authorities
        .iter()
        .cloned()
        .filter_map(|id| id)
        .collect();

    let validator_map = validators
        .into_iter()
        .zip(maybe_authorities.into_iter())
        .filter_map(|(k, v)| v.map(|v| (v, k)))
        .collect::<HashMap<AuthorityDiscoveryId, ValidatorId>>();

    let connections = connect_to_authorities(ctx, authorities).await;

    Ok(ConnectionRequest {
        validator_map,
        connections,
    })
}

async fn connect_to_authorities<Context: SubsystemContext>(
    ctx: &mut Context,
    validator_ids: Vec<AuthorityDiscoveryId>,
) -> mpsc::Receiver<(AuthorityDiscoveryId, PeerId)> {
    const PEERS_CAPACITY: usize = 8;

    let (connected, connected_rx) = mpsc::channel(PEERS_CAPACITY);

    ctx.send_message(AllMessages::NetworkBridge(
        NetworkBridgeMessage::ConnectToValidators {
            validator_ids,
            connected,
        },
    ))
    .await;

    connected_rx
}

/// Represents a discovered validator.
///
/// Result of [`ConnectionRequests::next`].
#[derive(Debug, PartialEq)]
pub struct DiscoveredValidator {
    /// The relay parent associated with the connection request that returned a result.
    pub relay_parent: Hash,
    /// The [`ValidatorId`] that was resolved.
    pub validator_id: ValidatorId,
    /// The [`PeerId`] associated to the validator id.
    pub peer_id: PeerId,
}

/// Used by [`ConnectionRequests::requests`] to map a [`ConnectionRequest`] item to a [`DiscoveredValidator`].
struct ConnectionRequestForRelayParent {
    request: ConnectionRequest,
    relay_parent: Hash,
}

impl stream::Stream for ConnectionRequestForRelayParent {
    type Item = DiscoveredValidator;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<Option<Self::Item>> {
        self.request.poll_next_unpin(cx).map(|r| {
            r.map(|(validator_id, peer_id)| DiscoveredValidator {
                validator_id,
                peer_id,
                relay_parent: self.relay_parent,
            })
        })
    }
}

/// A struct that assists performing multiple concurrent connection requests.
///
/// This allows concurrent connections to validator sets at different `relay_parents`.
/// Use [`ConnectionRequests::next`] to wait for results of the added connection requests.
#[derive(Default)]
pub struct ConnectionRequests {
    /// Connection requests relay_parent -> StreamUnordered token
    id_map: HashMap<Hash, usize>,

    /// Connection requests themselves.
    requests: StreamUnordered<ConnectionRequestForRelayParent>,
}

impl ConnectionRequests {
    /// Insert a new connection request.
    ///
    /// If a `ConnectionRequest` under a given `relay_parent` already exists it will
    /// be revoked and substituted with the given one.
    pub fn put(&mut self, relay_parent: Hash, request: ConnectionRequest) {
        self.remove(&relay_parent);
        let token = self.requests.insert(ConnectionRequestForRelayParent {
            relay_parent,
            request,
        });

        self.id_map.insert(relay_parent, token);
    }

    /// Remove a connection request by a given `relay_parent`.
    pub fn remove(&mut self, relay_parent: &Hash) {
        if let Some(token) = self.id_map.remove(relay_parent) {
            Pin::new(&mut self.requests).remove(token);
        }
    }

    /// Is a connection at this relay parent already present in the request
    pub fn contains_request(&self, relay_parent: &Hash) -> bool {
        self.id_map.contains_key(relay_parent)
    }

    /// Returns the next available connection request result.
    ///
    /// # Note
    ///
    /// When there are no active requests this will wait indefinitely, like an always pending future.
    pub async fn next(&mut self) -> DiscoveredValidator {
        loop {
            match self.requests.next().await {
                Some((StreamYield::Item(item), _)) => return item,
                // Ignore finished requests, they are required to be removed.
                Some((StreamYield::Finished(_), _)) => (),
                None => futures::pending!(),
            }
        }
    }
}

/// A pending connection request to validators.
/// This struct implements `Stream` to allow for asynchronous
/// discovery of validator addresses.
///
/// NOTE: the request will be revoked on drop.
#[must_use = "dropping a request will result in its immediate revokation"]
pub struct ConnectionRequest {
    validator_map: HashMap<AuthorityDiscoveryId, ValidatorId>,
    #[must_use = "streams do nothing unless polled"]
    connections: mpsc::Receiver<(AuthorityDiscoveryId, PeerId)>,
}

impl stream::Stream for ConnectionRequest {
    type Item = (ValidatorId, PeerId);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<Option<Self::Item>> {
        if self.validator_map.is_empty() {
            return Poll::Ready(None);
        }
        match Pin::new(&mut self.connections).poll_next(cx) {
            Poll::Ready(Some((id, peer_id))) => {
                if let Some(validator_id) = self.validator_map.remove(&id) {
                    return Poll::Ready(Some((validator_id, peer_id)));
                } else {
                    // unknown authority_id
                    // should be unreachable
                }
            }
            _ => {}
        }
        Poll::Pending
    }
}
