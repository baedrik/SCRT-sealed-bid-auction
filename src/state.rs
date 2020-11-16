use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

use std::collections::HashSet;

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
    pub sell_amount: Uint128,
    /// minimum bid that will be accepted
    pub minimum_bid: Uint128,
    /// amount of tokens currently consigned to auction escrow
    pub currently_consigned: Uint128,
    /// list of addresses of bidders
    pub bidders: HashSet<Vec<u8>>,
    /// true if the auction is closed
    pub is_completed: bool,
    /// true if all tokens for sale have been consigned to escrow
    pub tokens_consigned: bool,
    /// Optional text description of auction
    pub description: Option<String>,
}

/// storage key for auction state
pub static CONFIG_KEY: &[u8] = b"config";
/// Returns writable Singleton Storage associated with auction state
///
/// # Arguments
///
/// * `storage` - mutable reference to the contract's storage
pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}
/// Returns read-only Singleton Storage associated with auction state
///
/// # Arguments
///
/// * `storage` - reference to the contract's storage
pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

/// bid data
#[derive(Serialize, Deserialize)]
pub struct Bid {
    /// amount of bid
    pub amount: Uint128,
    /// time bid was placed
    pub timestamp: u64,
}
/// storage key for bids
pub const PREFIX_BIDS: &[u8] = b"bids";
/// Returns Bucket Storage associated with Bid type
///
/// # Arguments
///
/// * `storage` - mutable reference to the contract's storage
pub fn bids<S: Storage>(storage: &mut S) -> Bucket<S, Bid> {
    bucket(PREFIX_BIDS, storage)
}
/// Returns read-only Bucket Storage associated with Bid type
///
/// # Arguments
///
/// * `storage` - reference to the contract's storage
pub fn bids_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Bid> {
    bucket_read(PREFIX_BIDS, storage)
}
