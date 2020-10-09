use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

use std::collections::HashSet;

// struct containing token contract info
// hash: String -- code hash of the SNIP-20 token contract
// address: HumanAddr -- address of the SNIP-20 token contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    pub code_hash: String,
    pub address: HumanAddr,
}

// state of the auction holds:
// auction_addr: HumanAddr -- address of the auction contract
// seller: HumanAddr -- address of the auction owner
// sell_contract: ContractInfo -- code hash and address of sale token(s) SNIP-20 contract
// bid_contract: ContractInfo -- code hash and address of bid token(s) SNIP-20 contract
// sell_amount: Uint128 -- amount being sold
// minimum_bid: Uint128 -- minimum bid that will be accepted
// currently_consigned: Uint128 -- amount of sell tokens consigned so far
// bidders: HashSet<Vec<u8>> -- list of addresses with active bids, can't use CanonicalAddr,
//                              because it does not implement Eq or Hash
// is_completed: true if the auction is over
// tokens_consigned: true if seller has consigned the tokens for sale to the auction contract
// description: Option<String> -- optional description of the auction
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub auction_addr: HumanAddr,
    pub seller: HumanAddr,
    pub sell_contract: ContractInfo,
    pub bid_contract: ContractInfo,
    pub sell_amount: Uint128,
    pub minimum_bid: Uint128,
    pub currently_consigned: Uint128,
    pub bidders: HashSet<Vec<u8>>,
    pub is_completed: bool,
    pub tokens_consigned: bool,
    pub description: Option<String>,
}

// functions to save/read the state of the auction
pub static CONFIG_KEY: &[u8] = b"config";

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

// Bid data:
// amount: amount of tokens bid
// timestamp: time the bid was received, because ties go to the first to bid
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, JsonSchema)]
pub struct Bid {
    pub amount: Uint128,
    pub timestamp: u64,
}

// functions to return a bucket with all bids
pub const PREFIX_BIDS: &[u8] = b"bids";

pub fn bids<S: Storage>(storage: &mut S) -> Bucket<S, Bid> {
    bucket(PREFIX_BIDS, storage)
}

pub fn bids_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Bid> {
    bucket_read(PREFIX_BIDS, storage)
}
