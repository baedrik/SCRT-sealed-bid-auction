use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, StdError, Storage, Uint128,
};

use std::collections::HashSet;

use serde_json_wasm as serde_json;

use secret_toolkit::utils::{pad_handle_result, pad_query_result};

use crate::msg::{
    HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg, ResponseStatus,
    ResponseStatus::{Failure, Success},
    Token,
};
use crate::state::{bids, bids_read, config, config_read, Bid, State};

use chrono::NaiveDateTime;

// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
// response size
pub const BLOCK_SIZE: usize = 256;

////////////////////////////////////// Init ///////////////////////////////////////
/// Returns InitResult
///
/// Initializes the auction state and registers Receive function with sell and bid
/// token contracts
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - InitMsg passed in with the instantiation message
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    if msg.sell_amount == Uint128(0) {
        return Err(StdError::generic_err("Sell amount must be greater than 0"));
    }
    if msg.sell_contract.address == msg.bid_contract.address {
        return Err(StdError::generic_err(
            "Sell contract and bid contract must be different",
        ));
    }
    let state = State {
        auction_addr: env.contract.address,
        seller: env.message.sender,
        sell_contract: msg.sell_contract,
        bid_contract: msg.bid_contract,
        sell_amount: msg.sell_amount,
        minimum_bid: msg.minimum_bid,
        currently_consigned: Uint128(0),
        bidders: HashSet::new(),
        is_completed: false,
        tokens_consigned: false,
        description: msg.description,
    };

    config(&mut deps.storage).save(&state)?;

    // register receive with the bid/sell token contracts

    Ok(InitResponse {
        messages: vec![
            state
                .sell_contract
                .register_receive_msg(&env.contract_code_hash)?,
            state
                .bid_contract
                .register_receive_msg(&env.contract_code_hash)?,
        ],
        log: vec![],
    })
}

///////////////////////////////////// Handle //////////////////////////////////////
/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - HandleMsg passed in with the execute message
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    let response = match msg {
        HandleMsg::RetractBid { .. } => try_retract(deps, &env.message.sender),
        HandleMsg::Finalize { only_if_bids, .. } => try_finalize(deps, env, only_if_bids, false),
        HandleMsg::ReturnAll { .. } => try_finalize(deps, env, false, true),
        HandleMsg::Receive { from, amount, .. } => try_receive(deps, env, &from, amount),
        HandleMsg::ViewBid { .. } => try_view_bid(deps, &env.message.sender),
    };
    pad_handle_result(response, BLOCK_SIZE)
}

/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `bidder` - reference to address wanting to view its bid
fn try_view_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    bidder: &HumanAddr,
) -> HandleResult {
    let state = config_read(&deps.storage).load()?;

    let bidder_raw = &deps.api.canonical_address(bidder)?;
    let bidstore = bids_read(&deps.storage);
    let mut amount_bid: Option<Uint128> = None;
    let mut message = String::new();
    let status: ResponseStatus;

    if state.bidders.contains(&bidder_raw.as_slice().to_vec()) {
        let bid = bidstore.may_load(bidder_raw.as_slice())?;
        if let Some(found_bid) = bid {
            status = Success;
            amount_bid = Some(found_bid.amount);
            message.push_str(&format!(
                "Bid placed {} UTC",
                NaiveDateTime::from_timestamp(found_bid.timestamp as i64, 0)
                    .format("%Y-%m-%d %H:%M:%S")
            ));
        } else {
            status = Failure;
            message.push_str(&format!("No active bid for address: {}", bidder));
        }
    // no active bid found
    } else {
        status = Failure;
        message.push_str(&format!("No active bid for address: {}", bidder));
    }
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Bid {
            status,
            message,
            previous_bid: None,
            amount_bid,
            amount_returned: None,
        })?),
    })
}

/// Returns HandleResult
///
/// process the Receive message sent after either bid or sell token contract sent tokens to
/// auction escrow
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `from` - reference to address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: &HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let mut state = config_read(&deps.storage).load()?;
    if env.message.sender == state.sell_contract.address {
        try_consign(deps, from, amount, &mut state)
    } else if env.message.sender == state.bid_contract.address {
        try_bid(deps, env, from, amount, &mut state)
    } else {
        let message = format!(
            "Address: {} is not a token in this auction",
            env.message.sender
        );
        let resp = serde_json::to_string(&HandleAnswer::Status {
            status: Failure,
            message,
        })
        .unwrap();

        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        })
    }
}

/// Returns HandleResult
///
/// process the attempt to consign sale tokens to auction escrow
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `owner` - reference to address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
fn try_consign<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    owner: &HumanAddr,
    amount: Uint128,
    state: &mut State,
) -> HandleResult {
    // if not the auction owner, send the tokens back
    if *owner != state.seller {
        let message = String::from(
            "Only auction creator can consign tokens for sale.  Your tokens have been returned",
        );

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Your tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if tokens to be sold have already been consigned, return these tokens
    if state.tokens_consigned {
        let message = String::from(
            "Tokens to be sold have already been consigned. Your tokens have been returned",
        );

        let resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: Some(state.currently_consigned),
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![state.sell_contract.transfer_msg(owner, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }

    let consign_total = state.currently_consigned + amount;
    let mut log_msg = String::new();
    let mut cos_msg = Vec::new();
    let status: ResponseStatus;
    let mut excess: Option<Uint128> = None;
    let mut needed: Option<Uint128> = None;
    // if consignment amount < auction sell amount, ask for remaining balance
    if consign_total < state.sell_amount {
        state.currently_consigned = consign_total;
        needed = Some((state.sell_amount - consign_total).unwrap());
        status = Failure;
        log_msg.push_str(
            "You have not consigned the full amount to be sold.  You need to consign additional \
             tokens",
        );
    // all tokens to be sold have been consigned
    } else {
        state.tokens_consigned = true;
        state.currently_consigned = state.sell_amount;
        status = Success;
        log_msg.push_str("Tokens to be sold have been consigned to the auction");
        // if consigned more than needed, return excess tokens
        if consign_total > state.sell_amount {
            excess = Some((consign_total - state.sell_amount).unwrap());
            cos_msg.push(state.sell_contract.transfer_msg(owner, excess.unwrap())?);
            log_msg.push_str(".  Excess tokens have been returned");
        }
    }
    config(&mut deps.storage).save(state)?;

    let resp = serde_json::to_string(&HandleAnswer::Consign {
        status,
        message: log_msg,
        amount_consigned: Some(state.currently_consigned),
        amount_needed: needed,
        amount_returned: excess,
    })
    .unwrap();

    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

/// Returns HandleResult
///
/// process the bid attempt
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `bidder` - reference to address of owner of tokens sent to escrow
/// * `amount` - Uint128 amount sent to escrow
/// * `state` - mutable reference to auction state
fn try_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bidder: &HumanAddr,
    amount: Uint128,
    state: &mut State,
) -> HandleResult {
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Bid tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![state.bid_contract.transfer_msg(bidder, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // don't accept a 0 bid
    if amount == Uint128(0) {
        let message = String::from("Bid must be greater than 0");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: None,
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if bid is less than the minimum accepted bid, send the tokens back
    if amount < state.minimum_bid {
        let message =
            String::from("Bid was less than minimum allowed.  Bid tokens have been returned");

        let resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        return Ok(HandleResponse {
            messages: vec![state.bid_contract.transfer_msg(bidder, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    let mut return_amount: Option<Uint128> = None;
    let bidder_raw = &deps.api.canonical_address(bidder)?;
    let bidstore = bids_read(&deps.storage);

    // if there is an active bid from this address
    if state.bidders.contains(&bidder_raw.as_slice().to_vec()) {
        let bid = bidstore.may_load(bidder_raw.as_slice())?;
        if let Some(old_bid) = bid {
            // if new bid is <= the old bid, keep old bid and return this one
            if amount <= old_bid.amount {
                let message = String::from(
                    "New bid less than or equal to previous bid. Newly bid tokens have been \
                     returned",
                );

                let resp = serde_json::to_string(&HandleAnswer::Bid {
                    status: Failure,
                    message,
                    previous_bid: Some(old_bid.amount),
                    amount_bid: None,
                    amount_returned: Some(amount),
                })
                .unwrap();

                return Ok(HandleResponse {
                    messages: vec![state.bid_contract.transfer_msg(bidder, amount)?],
                    log: vec![log("response", resp)],
                    data: None,
                });
            // new bid is larger, save the new bid, and return the old one, so mark for return
            } else {
                return_amount = Some(old_bid.amount);
            }
        }
    // address did not have an active bid
    } else {
        // insert in list of bidders and save
        state.bidders.insert(bidder_raw.as_slice().to_vec());
        config(&mut deps.storage).save(&state)?;
    }
    let new_bid = Bid {
        amount,
        timestamp: env.block.time,
    };
    let mut bid_save = bids(&mut deps.storage);
    bid_save.save(bidder_raw.as_slice(), &new_bid)?;

    let mut message = String::from("Bid accepted");
    let mut cos_msg = Vec::new();

    // if need to return the old bid
    if let Some(returned) = return_amount {
        cos_msg.push(state.bid_contract.transfer_msg(bidder, returned)?);
        message.push_str(". Previously bid tokens have been returned");
    }
    let resp = serde_json::to_string(&HandleAnswer::Bid {
        status: Success,
        message,
        previous_bid: None,
        amount_bid: Some(new_bid.amount),
        amount_returned: return_amount,
    })
    .unwrap();

    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

/// Returns HandleResult
///
/// attempt to retract current bid
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `bidder` - reference to address of bidder
fn try_retract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    bidder: &HumanAddr,
) -> HandleResult {
    let mut state = config_read(&deps.storage).load()?;

    let bidder_raw = &deps.api.canonical_address(bidder)?;
    let mut bids = bids(&mut deps.storage);
    let mut cos_msg = Vec::new();
    let sent: Option<Uint128>;
    let mut log_msg = String::new();
    let status: ResponseStatus;
    // if there was a active bid from this address, remove the bid and return tokens
    if state.bidders.contains(&bidder_raw.as_slice().to_vec()) {
        let bid = bids.may_load(bidder_raw.as_slice())?;
        if let Some(old_bid) = bid {
            bids.remove(bidder_raw.as_slice());
            state.bidders.remove(&bidder_raw.as_slice().to_vec());
            config(&mut deps.storage).save(&state)?;
            cos_msg.push(state.bid_contract.transfer_msg(bidder, old_bid.amount)?);
            status = Success;
            sent = Some(old_bid.amount);
            log_msg.push_str("Bid retracted.  Tokens have been returned");
        } else {
            status = Failure;
            sent = None;
            log_msg.push_str(&format!("No active bid for address: {}", bidder));
        }
    // no active bid found
    } else {
        status = Failure;
        sent = None;
        log_msg.push_str(&format!("No active bid for address: {}", bidder));
    }
    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RetractBid {
            status,
            message: log_msg,
            amount_returned: sent,
        })?),
    })
}

/// Returns HandleResult
///
/// closes the auction and sends all the tokens in escrow to where they belong
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `only_if_bids` - true if auction should stay open if there are no bids
/// * `return_all` - true if being called from the return_all fallback plan
fn try_finalize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    only_if_bids: bool,
    return_all: bool,
) -> HandleResult {
    let mut state = config_read(&deps.storage).load()?;
    // can only do a return_all if the auction is closed
    if return_all && !state.is_completed {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from(
                    "return_all can only be executed after the auction has ended",
                ),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    // if not the auction owner, can't finalize, but you can return_all
    if !return_all && env.message.sender != state.seller {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from("Only auction creator can finalize the sale"),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    // if there are no active bids, and owner only wants to close if bids
    if !state.is_completed && only_if_bids && state.bidders.is_empty() {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::CloseAuction {
                status: Failure,
                message: String::from("Did not close because there are no active bids"),
                winning_bid: None,
                amount_returned: None,
            })?),
        });
    }
    let mut cos_msg = Vec::new();
    let mut bids = bids(&mut deps.storage);
    let mut update_state = false;
    let mut winning_amount: Option<Uint128> = None;
    let mut amount_returned: Option<Uint128> = None;

    let no_bids = state.bidders.is_empty();
    // if there were bids
    if !no_bids {
        // load all the bids
        struct OwnedBid {
            pub bidder: CanonicalAddr,
            pub bid: Bid,
        }
        let mut bid_list: Vec<OwnedBid> = Vec::new();
        for bidder in &state.bidders {
            let bid = bids.may_load(bidder.as_slice())?;
            if let Some(found_bid) = bid {
                bid_list.push(OwnedBid {
                    bidder: CanonicalAddr::from(bidder.as_slice()),
                    bid: found_bid,
                });
            }
        }
        // closing an auction that has been fully consigned
        if state.tokens_consigned && !state.is_completed {
            bid_list.sort_by(|a, b| {
                a.bid
                    .amount
                    .cmp(&b.bid.amount)
                    .then(b.bid.timestamp.cmp(&a.bid.timestamp))
            });
            // if there was a winner, swap the tokens
            if let Some(winning_bid) = bid_list.pop() {
                cos_msg.push(
                    state
                        .bid_contract
                        .transfer_msg(&state.seller, winning_bid.bid.amount)?,
                );
                cos_msg.push(state.sell_contract.transfer_msg(
                    &deps.api.human_address(&winning_bid.bidder)?,
                    state.sell_amount,
                )?);
                state.currently_consigned = Uint128(0);
                update_state = true;
                winning_amount = Some(winning_bid.bid.amount);
                bids.remove(&winning_bid.bidder.as_slice());
                state
                    .bidders
                    .remove(&winning_bid.bidder.as_slice().to_vec());
            }
        }
        // loops through all remaining bids to return them to the bidders
        for losing_bid in &bid_list {
            cos_msg.push(state.bid_contract.transfer_msg(
                &deps.api.human_address(&losing_bid.bidder)?,
                losing_bid.bid.amount,
            )?);
            bids.remove(&losing_bid.bidder.as_slice());
            update_state = true;
            state.bidders.remove(&losing_bid.bidder.as_slice().to_vec());
        }
    }
    // return any tokens that have been consigned to the auction owner (can happen if owner
    // finalized the auction before consigning the full sale amount or if there were no bids)
    if state.currently_consigned > Uint128(0) {
        cos_msg.push(
            state
                .sell_contract
                .transfer_msg(&state.seller, state.currently_consigned)?,
        );
        if !return_all {
            amount_returned = Some(state.currently_consigned);
        }
        state.currently_consigned = Uint128(0);
        update_state = true;
    }
    // mark that auction had ended
    if !state.is_completed {
        state.is_completed = true;
        update_state = true;
    }
    if update_state {
        config(&mut deps.storage).save(&state)?;
    }

    let log_msg = if winning_amount.is_some() {
        "Sale finalized.  You have been sent the winning bid tokens".to_string()
    } else if amount_returned.is_some() {
        let cause = if !state.tokens_consigned {
            " because you did not consign the full sale amount"
        } else if no_bids {
            " because there were no active bids"
        } else {
            ""
        };
        format!(
            "Auction closed.  You have been returned the consigned tokens{}",
            cause
        )
    } else if return_all {
        "Outstanding funds have been returned".to_string()
    } else {
        "Auction has been closed".to_string()
    };
    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::CloseAuction {
            status: Success,
            message: log_msg,
            winning_bid: winning_amount,
            amount_returned,
        })?),
    })
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::AuctionInfo { .. } => try_query_info(deps),
    };
    pad_query_result(response, BLOCK_SIZE)
}

/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
fn try_query_info<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let state = config_read(&deps.storage).load()?;

    // get sell token info
    let sell_token_info = state.sell_contract.token_info_query(&deps.querier)?;
    // get bid token info
    let bid_token_info = state.bid_contract.token_info_query(&deps.querier)?;

    // build status string
    let status = if state.is_completed {
        let locked = if !state.bidders.is_empty() || state.currently_consigned > Uint128(0) {
            ", but found outstanding balances.  Please run either retract_bid to \
                retrieve your non-winning bid, or return_all to return all outstanding bids/\
                consignment."
        } else {
            ""
        };
        format!("Closed{}", locked)
    } else {
        let consign = if !state.tokens_consigned { " NOT" } else { "" };
        format!(
            "Accepting bids: Token(s) to be sold have{} been consigned to the auction",
            consign
        )
    };

    to_binary(&QueryAnswer::AuctionInfo {
        sell_token: Token {
            contract_address: state.sell_contract.address,
            token_info: sell_token_info,
        },
        bid_token: Token {
            contract_address: state.bid_contract.address,
            token_info: bid_token_info,
        },
        sell_amount: state.sell_amount,
        minimum_bid: state.minimum_bid,
        description: state.description,
        auction_address: state.auction_addr,
        status,
    })
}
