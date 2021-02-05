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

use sp_runtime::RuntimeDebug;
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    mem,
    prelude::*,
};
use xcm::v0::{AssetInstance, MultiAsset, MultiLocation};

/// Classification of an asset being concrete or abstract.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug)]
pub enum AssetId {
    Concrete(MultiLocation),
    Abstract(Vec<u8>),
}

impl AssetId {
    /// Prepend a MultiLocation to a concrete asset, giving it a new root location.
    pub fn reanchor(&mut self, prepend: &MultiLocation) -> Result<(), ()> {
        if let AssetId::Concrete(ref mut l) = self {
            l.prepend_with(prepend.clone()).map_err(|_| ())?;
        }
        Ok(())
    }
}

/// List of concretely identified fungible and non-fungible assets.
#[derive(Default, Clone, RuntimeDebug)]
pub struct Assets {
    pub fungible: BTreeMap<AssetId, u128>,
    pub non_fungible: BTreeSet<(AssetId, AssetInstance)>,
}

impl From<Vec<MultiAsset>> for Assets {
    fn from(assets: Vec<MultiAsset>) -> Assets {
        let mut result = Self::default();
        for asset in assets.into_iter() {
            result.saturating_subsume(asset)
        }
        result
    }
}

impl From<Assets> for Vec<MultiAsset> {
    fn from(a: Assets) -> Self {
        a.into_assets_iter().collect()
    }
}

impl Assets {
    /// An iterator over the fungible assets.
    pub fn fungible_assets_iter<'a>(&'a self) -> impl Iterator<Item = MultiAsset> + 'a {
        self.fungible.iter().map(|(id, &amount)| match id.clone() {
            AssetId::Concrete(id) => MultiAsset::ConcreteFungible { id, amount },
            AssetId::Abstract(id) => MultiAsset::AbstractFungible { id, amount },
        })
    }

    /// An iterator over the non-fungible assets.
    pub fn non_fungible_assets_iter<'a>(&'a self) -> impl Iterator<Item = MultiAsset> + 'a {
        self.non_fungible
            .iter()
            .map(|&(ref class, ref instance)| match class.clone() {
                AssetId::Concrete(class) => MultiAsset::ConcreteNonFungible {
                    class,
                    instance: instance.clone(),
                },
                AssetId::Abstract(class) => MultiAsset::AbstractNonFungible {
                    class,
                    instance: instance.clone(),
                },
            })
    }

    /// An iterator over all assets.
    pub fn into_assets_iter(self) -> impl Iterator<Item = MultiAsset> {
        let fungible = self.fungible.into_iter().map(|(id, amount)| match id {
            AssetId::Concrete(id) => MultiAsset::ConcreteFungible { id, amount },
            AssetId::Abstract(id) => MultiAsset::AbstractFungible { id, amount },
        });
        let non_fungible = self
            .non_fungible
            .into_iter()
            .map(|(id, instance)| match id {
                AssetId::Concrete(class) => MultiAsset::ConcreteNonFungible { class, instance },
                AssetId::Abstract(class) => MultiAsset::AbstractNonFungible { class, instance },
            });
        fungible.chain(non_fungible)
    }

    /// An iterator over all assets.
    pub fn assets_iter<'a>(&'a self) -> impl Iterator<Item = MultiAsset> + 'a {
        let fungible = self.fungible_assets_iter();
        let non_fungible = self.non_fungible_assets_iter();
        fungible.chain(non_fungible)
    }

    /// Modify `self` to include a `MultiAsset`, saturating if necessary.
    /// Only works on concretely identified assets; wildcards will be swallowed without error.
    pub fn saturating_subsume(&mut self, asset: MultiAsset) {
        match asset {
            MultiAsset::ConcreteFungible { id, amount } => {
                self.saturating_subsume_fungible(AssetId::Concrete(id), amount);
            }
            MultiAsset::AbstractFungible { id, amount } => {
                self.saturating_subsume_fungible(AssetId::Abstract(id), amount);
            }
            MultiAsset::ConcreteNonFungible { class, instance } => {
                self.saturating_subsume_non_fungible(AssetId::Concrete(class), instance);
            }
            MultiAsset::AbstractNonFungible { class, instance } => {
                self.saturating_subsume_non_fungible(AssetId::Abstract(class), instance);
            }
            _ => (),
        }
    }

    /// Modify `self` to include a new fungible asset by `id` and `amount`,
    /// saturating if necessary.
    pub fn saturating_subsume_fungible(&mut self, id: AssetId, amount: u128) {
        self.fungible
            .entry(id)
            .and_modify(|e| *e = e.saturating_add(amount))
            .or_insert(amount);
    }

    /// Modify `self` to include a new non-fungible asset by `class` and `instance`.
    pub fn saturating_subsume_non_fungible(&mut self, class: AssetId, instance: AssetInstance) {
        self.non_fungible.insert((class, instance));
    }

    /// Alter any concretely identified assets according to the given `MultiLocation`.
    ///
    /// WARNING: For now we consider this infallible and swallow any errors. It is thus the caller's responsibility to
    /// ensure that any internal asset IDs are able to be prepended without overflow.
    pub fn reanchor(&mut self, prepend: &MultiLocation) {
        let mut fungible = Default::default();
        mem::swap(&mut self.fungible, &mut fungible);
        self.fungible = fungible
            .into_iter()
            .map(|(mut id, amount)| {
                let _ = id.reanchor(prepend);
                (id, amount)
            })
            .collect();
        let mut non_fungible = Default::default();
        mem::swap(&mut self.non_fungible, &mut non_fungible);
        self.non_fungible = non_fungible
            .into_iter()
            .map(|(mut class, inst)| {
                let _ = class.reanchor(prepend);
                (class, inst)
            })
            .collect();
    }

    /// Return the assets in `self`, but (asset-wise) of no greater value than `assets`.
    ///
    /// Result is undefined if `assets` includes elements which match to the same asset more than once.
    ///
    /// Example:
    ///
    /// ```
    /// use xcm_executor::Assets;
    /// use xcm::v0::{MultiAsset, MultiLocation};
    /// let assets_i_have: Assets = vec![
    /// 	MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount: 100 },
    /// 	MultiAsset::AbstractFungible { id: vec![0], amount: 100 },
    /// ].into();
    /// let assets_they_want: Assets = vec![
    /// 	MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount: 200 },
    /// 	MultiAsset::AbstractFungible { id: vec![0], amount: 50 },
    /// ].into();
    ///
    /// let assets_we_can_trade: Assets = assets_i_have.min(assets_they_want.assets_iter());
    /// assert_eq!(assets_we_can_trade.into_assets_iter().collect::<Vec<_>>(), vec![
    /// 	MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount: 100 },
    /// 	MultiAsset::AbstractFungible { id: vec![0], amount: 50 },
    /// ]);
    /// ```
    pub fn min<'a, M, I>(&self, assets: I) -> Self
    where
        M: 'a + sp_std::borrow::Borrow<MultiAsset>,
        I: IntoIterator<Item = M>,
    {
        let mut result = Assets::default();
        for asset in assets.into_iter() {
            match asset.borrow() {
                MultiAsset::None => (),
                MultiAsset::All => return self.clone(),
                MultiAsset::AllFungible => {
                    // Replace `result.fungible` with all fungible assets,
                    // keeping `result.non_fungible` the same.
                    result = Assets {
                        fungible: self.fungible.clone(),
                        non_fungible: result.non_fungible,
                    }
                }
                MultiAsset::AllNonFungible => {
                    // Replace `result.non_fungible` with all non-fungible assets,
                    // keeping `result.fungible` the same.
                    result = Assets {
                        fungible: result.fungible,
                        non_fungible: self.non_fungible.clone(),
                    }
                }
                MultiAsset::AllAbstractFungible { id } => {
                    for asset in self.fungible_assets_iter() {
                        match &asset {
                            MultiAsset::AbstractFungible { id: identifier, .. } => {
                                if id == identifier {
                                    result.saturating_subsume(asset)
                                }
                            }
                            _ => (),
                        }
                    }
                }
                MultiAsset::AllAbstractNonFungible { class } => {
                    for asset in self.non_fungible_assets_iter() {
                        match &asset {
                            MultiAsset::AbstractNonFungible { class: c, .. } => {
                                if class == c {
                                    result.saturating_subsume(asset)
                                }
                            }
                            _ => (),
                        }
                    }
                }
                MultiAsset::AllConcreteFungible { id } => {
                    for asset in self.fungible_assets_iter() {
                        match &asset {
                            MultiAsset::ConcreteFungible { id: identifier, .. } => {
                                if id == identifier {
                                    result.saturating_subsume(asset)
                                }
                            }
                            _ => (),
                        }
                    }
                }
                MultiAsset::AllConcreteNonFungible { class } => {
                    for asset in self.non_fungible_assets_iter() {
                        match &asset {
                            MultiAsset::ConcreteNonFungible { class: c, .. } => {
                                if class == c {
                                    result.saturating_subsume(asset)
                                }
                            }
                            _ => (),
                        }
                    }
                }
                x @ MultiAsset::ConcreteFungible { .. }
                | x @ MultiAsset::AbstractFungible { .. } => {
                    let (id, amount) = match x {
                        MultiAsset::ConcreteFungible { id, amount } => {
                            (AssetId::Concrete(id.clone()), *amount)
                        }
                        MultiAsset::AbstractFungible { id, amount } => {
                            (AssetId::Abstract(id.clone()), *amount)
                        }
                        _ => unreachable!(),
                    };
                    if let Some(v) = self.fungible.get(&id) {
                        result.saturating_subsume_fungible(id, amount.min(*v));
                    }
                }
                x @ MultiAsset::ConcreteNonFungible { .. }
                | x @ MultiAsset::AbstractNonFungible { .. } => {
                    let (class, instance) = match x {
                        MultiAsset::ConcreteNonFungible { class, instance } => {
                            (AssetId::Concrete(class.clone()), instance.clone())
                        }
                        MultiAsset::AbstractNonFungible { class, instance } => {
                            (AssetId::Abstract(class.clone()), instance.clone())
                        }
                        _ => unreachable!(),
                    };
                    let item = (class, instance);
                    if self.non_fungible.contains(&item) {
                        result.non_fungible.insert(item);
                    }
                }
            }
        }
        result
    }

    /// Take all possible assets up to `assets` from `self`, mutating `self` and returning the
    /// assets taken.
    ///
    /// Wildcards work.
    ///
    /// Example:
    ///
    /// ```
    /// use xcm_executor::Assets;
    /// use xcm::v0::{MultiAsset, MultiLocation};
    /// let mut assets_i_have: Assets = vec![
    /// 	MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount: 100 },
    /// 	MultiAsset::AbstractFungible { id: vec![0], amount: 100 },
    /// ].into();
    /// let assets_they_want = vec![
    /// 	MultiAsset::AllAbstractFungible { id: vec![0] },
    /// ];
    ///
    /// let assets_they_took: Assets = assets_i_have.saturating_take(assets_they_want);
    /// assert_eq!(assets_they_took.into_assets_iter().collect::<Vec<_>>(), vec![
    /// 	MultiAsset::AbstractFungible { id: vec![0], amount: 100 },
    /// ]);
    /// assert_eq!(assets_i_have.into_assets_iter().collect::<Vec<_>>(), vec![
    /// 	MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount: 100 },
    /// ]);
    /// ```
    pub fn saturating_take<I>(&mut self, assets: I) -> Assets
    where
        I: IntoIterator<Item = MultiAsset>,
    {
        let mut result = Assets::default();
        for asset in assets.into_iter() {
            match asset {
                MultiAsset::None => (),
                MultiAsset::All => return self.swapped(Assets::default()),
                MultiAsset::AllFungible => {
                    // Remove all fungible assets, and copy them into `result`.
                    let fungible = mem::replace(&mut self.fungible, Default::default());
                    fungible.into_iter().for_each(|(id, amount)| {
                        result.saturating_subsume_fungible(id, amount);
                    })
                }
                MultiAsset::AllNonFungible => {
                    // Remove all non-fungible assets, and copy them into `result`.
                    let non_fungible = mem::replace(&mut self.non_fungible, Default::default());
                    non_fungible.into_iter().for_each(|(class, instance)| {
                        result.saturating_subsume_non_fungible(class, instance);
                    });
                }
                x @ MultiAsset::AllAbstractFungible { .. }
                | x @ MultiAsset::AllConcreteFungible { .. } => {
                    let id = match x {
                        MultiAsset::AllConcreteFungible { id } => AssetId::Concrete(id),
                        MultiAsset::AllAbstractFungible { id } => AssetId::Abstract(id),
                        _ => unreachable!(),
                    };
                    // At the end of this block, we will be left with only the non-matching fungibles.
                    let mut non_matching_fungibles = BTreeMap::<AssetId, u128>::new();
                    let fungible = mem::replace(&mut self.fungible, Default::default());
                    fungible.into_iter().for_each(|(iden, amount)| {
                        if iden == id {
                            result.saturating_subsume_fungible(iden, amount);
                        } else {
                            non_matching_fungibles.insert(iden, amount);
                        }
                    });
                    self.fungible = non_matching_fungibles;
                }
                x @ MultiAsset::AllAbstractNonFungible { .. }
                | x @ MultiAsset::AllConcreteNonFungible { .. } => {
                    let class = match x {
                        MultiAsset::AllConcreteNonFungible { class } => AssetId::Concrete(class),
                        MultiAsset::AllAbstractNonFungible { class } => AssetId::Abstract(class),
                        _ => unreachable!(),
                    };
                    // At the end of this block, we will be left with only the non-matching non-fungibles.
                    let mut non_matching_non_fungibles =
                        BTreeSet::<(AssetId, AssetInstance)>::new();
                    let non_fungible = mem::replace(&mut self.non_fungible, Default::default());
                    non_fungible.into_iter().for_each(|(c, instance)| {
                        if class == c {
                            result.saturating_subsume_non_fungible(c, instance);
                        } else {
                            non_matching_non_fungibles.insert((c, instance));
                        }
                    });
                    self.non_fungible = non_matching_non_fungibles;
                }
                x @ MultiAsset::ConcreteFungible { .. }
                | x @ MultiAsset::AbstractFungible { .. } => {
                    let (id, amount) = match x {
                        MultiAsset::ConcreteFungible { id, amount } => {
                            (AssetId::Concrete(id), amount)
                        }
                        MultiAsset::AbstractFungible { id, amount } => {
                            (AssetId::Abstract(id), amount)
                        }
                        _ => unreachable!(),
                    };
                    // remove the maxmimum possible up to id/amount from self, add the removed onto
                    // result
                    let maybe_value = self.fungible.get(&id);
                    if let Some(&e) = maybe_value {
                        if e > amount {
                            self.fungible.insert(id.clone(), e - amount);
                            result.saturating_subsume_fungible(id, amount);
                        } else {
                            self.fungible.remove(&id);
                            result.saturating_subsume_fungible(id, e.clone());
                        }
                    }
                }
                x @ MultiAsset::ConcreteNonFungible { .. }
                | x @ MultiAsset::AbstractNonFungible { .. } => {
                    let (class, instance) = match x {
                        MultiAsset::ConcreteNonFungible { class, instance } => {
                            (AssetId::Concrete(class), instance)
                        }
                        MultiAsset::AbstractNonFungible { class, instance } => {
                            (AssetId::Abstract(class), instance)
                        }
                        _ => unreachable!(),
                    };
                    // remove the maxmimum possible up to id/amount from self, add the removed onto
                    // result
                    if let Some(entry) = self.non_fungible.take(&(class, instance)) {
                        result.non_fungible.insert(entry);
                    }
                }
            }
        }
        result
    }

    /// Swaps two mutable Assets, without deinitializing either one.
    pub fn swapped(&mut self, mut with: Assets) -> Self {
        mem::swap(&mut *self, &mut with);
        with
    }
}
