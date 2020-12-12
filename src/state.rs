use std::{any::type_name, collections::HashSet};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit::serialization::{Bincode2, Serde};

use crate::msg::ContractInfo;

/// state of the auction
#[derive(Serialize, Deserialize)]
pub struct State {
    /// address of auction contract
    pub auction_addr: HumanAddr,
    /// address of auction owner
    pub seller: HumanAddr,
    /// code hash and address of sell token contract
    pub sell_contract: ContractInfo,
    /// code hash and address of bid token contract
    pub bid_contract: ContractInfo,
    /// amount of tokens for sale
    pub sell_amount: u128,
    /// minimum bid that will be accepted
    pub minimum_bid: u128,
    /// amount of tokens currently consigned to auction escrow
    pub currently_consigned: u128,
    /// list of addresses of bidders
    pub bidders: HashSet<Vec<u8>>,
    /// true if the auction is closed
    pub is_completed: bool,
    /// true if all tokens for sale have been consigned to escrow
    pub tokens_consigned: bool,
    /// Optional text description of auction
    pub description: Option<String>,
}

/// bid data
#[derive(Serialize, Deserialize)]
pub struct Bid {
    /// amount of bid
    pub amount: u128,
    /// time bid was placed
    pub timestamp: u64,
}

/// Returns StdResult<()> resulting from saving an item to storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

/// Removes an item from storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn remove<S: Storage>(storage: &mut S, key: &[u8]) {
    storage.remove(key);
}

/// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
/// StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

/// Returns StdResult<Option<T>> from retrieving the item with the specified key.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Bincode2::deserialize(&value).map(Some),
        None => Ok(None),
    }
}
