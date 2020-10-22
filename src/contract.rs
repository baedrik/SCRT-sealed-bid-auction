use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, QueryRequest, QueryResult, StdError, StdResult, Storage, Uint128,
    WasmQuery,
};

use std::collections::HashSet;

use serde_json_wasm as serde_json;

use crate::msg::{
    pad_log_str, space_pad, HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg, RegisterMsg,
    ResponseStatus,
    ResponseStatus::{Failure, Success},
    Token, TokenQuery, TransferMsg,
};
use crate::state::{bids, bids_read, config, config_read, Bid, State};

use chrono::NaiveDateTime;

// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
// response size
pub const RESPONSE_BLOCK_SIZE: usize = 256;

////////////////////////////////////// Init ///////////////////////////////////////
// initialize the auction and register receiving function with token contracts
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.sell_amount == Uint128(0) {
        return Err(StdError::generic_err("Sell amount must be greater than 0"));
    }
    if msg.sell_contract.address == msg.bid_contract.address {
        return Err(StdError::generic_err(
            "Sell contract and bid contract must be different",
        ));
    }
    let state = State {
        auction_addr: env.contract.address.clone(),
        seller: env.message.sender,
        sell_contract: msg.sell_contract.clone(),
        bid_contract: msg.bid_contract.clone(),
        sell_amount: msg.sell_amount,
        minimum_bid: msg.minimum_bid,
        currently_consigned: Uint128(0),
        bidders: HashSet::new(),
        is_completed: false,
        tokens_consigned: false,
        description: msg.description,
    };

    config(&mut deps.storage).save(&state)?;

    // register receive functions with the bid/sell token contracts
    let register_rec_msg = RegisterMsg {
        code_hash: env.contract_code_hash,
    };
    let mut cosmsg = Vec::new();
    cosmsg.push(
        register_rec_msg
            .clone()
            .into_cosmos_msg(msg.sell_contract.code_hash, msg.sell_contract.address)?,
    );
    cosmsg.push(
        register_rec_msg.into_cosmos_msg(msg.bid_contract.code_hash, msg.bid_contract.address)?,
    );

    Ok(InitResponse {
        messages: cosmsg,
        log: vec![],
    })
}

///////////////////////////////////// Handle //////////////////////////////////////
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let response = match msg {
        HandleMsg::RetractBid { .. } => try_retract(deps, &env.message.sender),
        HandleMsg::Finalize { only_if_bids, .. } => try_finalize(deps, env, only_if_bids, false),
        HandleMsg::ReturnAll { .. } => try_finalize(deps, env, false, true),
        HandleMsg::Receive { from, amount, .. } => try_receive(deps, env, &from, amount),
        HandleMsg::ViewBid { .. } => try_view_bid(deps, &env.message.sender),
    };

    response.map(|mut response| {
        response.data = response.data.map(|mut data| {
            space_pad(RESPONSE_BLOCK_SIZE, &mut data.0);
            data
        });
        response
    })
}

// try_view_bid -- Allows the message sender to view their current active bid
//     bidder: &HumanAddr -- address of the message sender
// return value: StdResult<HandleResponse>
fn try_view_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    bidder: &HumanAddr,
) -> StdResult<HandleResponse> {
    let state = config_read(&deps.storage).load()?;

    let bidder_raw = &deps.api.canonical_address(&bidder.clone())?;
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
            message.push_str(&format!("No active bid for address: {}", bidder.clone()));
        }
    // no active bid found
    } else {
        status = Failure;
        message.push_str(&format!("No active bid for address: {}", bidder.clone()));
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

// try_receive -- processes the callbacks from token contracts after receiving tokens
//     from: &HumanAddr -- owner address  of the tokens sent to the auction
//     amount: Uint128 -- amount of tokens sent to auction
// return value: StdResult<HandleResponse>
fn try_receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    from: &HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
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
        let mut resp = serde_json::to_string(&HandleAnswer::Status {
            status: Failure,
            message,
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);

        Ok(HandleResponse {
            messages: vec![],
            log: vec![log("response", resp)],
            data: None,
        })
    }
}

// try_consign -- consigns tokens for sale.  If consigning more than the sale amount, any extra
//                tokens are returned
//     owner: &HumanAddr -- owner of tokens sent for consignment
//     amount: Uint128 -- amount to be consigned
//     state: &mut State -- auction state
// return value: StdResult<HandleResponse>
fn try_consign<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    owner: &HumanAddr,
    amount: Uint128,
    state: &mut State,
) -> StdResult<HandleResponse> {
    // if not the auction owner, send the tokens back
    if *owner != state.seller {
        let message = String::from(
            "Only auction creator can consign tokens for sale.  Your tokens have been returned",
        );

        let mut resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
        return Ok(HandleResponse {
            messages: vec![transfer_tokens_msg(
                state.sell_contract.code_hash.clone(),
                &state.sell_contract.address.clone(),
                &owner.clone(),
                amount,
            )?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Your tokens have been returned");

        let mut resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: None,
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
        return Ok(HandleResponse {
            messages: vec![transfer_tokens_msg(
                state.sell_contract.code_hash.clone(),
                &state.sell_contract.address.clone(),
                &owner.clone(),
                amount,
            )?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // if tokens to be sold have already been consigned, return these tokens
    if state.tokens_consigned {
        let message = String::from(
            "Tokens to be sold have already been consigned. Your tokens have been returned",
        );

        let mut resp = serde_json::to_string(&HandleAnswer::Consign {
            status: Failure,
            message,
            amount_consigned: Some(state.currently_consigned),
            amount_needed: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
        return Ok(HandleResponse {
            messages: vec![transfer_tokens_msg(
                state.sell_contract.code_hash.clone(),
                &state.sell_contract.address.clone(),
                &owner.clone(),
                amount,
            )?],
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
            cos_msg.push(transfer_tokens_msg(
                state.sell_contract.code_hash.clone(),
                &state.sell_contract.address.clone(),
                &owner.clone(),
                excess.unwrap(),
            )?);
            log_msg.push_str(".  Excess tokens have been returned");
        }
    }
    config(&mut deps.storage).save(state)?;

    let mut resp = serde_json::to_string(&HandleAnswer::Consign {
        status,
        message: log_msg,
        amount_consigned: Some(state.currently_consigned),
        amount_needed: needed,
        amount_returned: excess,
    })
    .unwrap();

    pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);

    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

// try_bid - places a bid.  If there was a previous bid, return the smaller bid to the bidder.  If
//           they are the same amount, keep the bid placed first so that it has the earlier
//           timestamp
//     bidder: &HumanAddr -- owner of tokens sent as a bid
//     amount: Uint128 -- amount bid
//     state: &mut State --  auction state
// return value: StdResult<HandleResponse>
fn try_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bidder: &HumanAddr,
    amount: Uint128,
    state: &mut State,
) -> StdResult<HandleResponse> {
    // if auction is over, send the tokens back
    if state.is_completed {
        let message = String::from("Auction has ended. Bid tokens have been returned");

        let mut resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
        return Ok(HandleResponse {
            messages: vec![transfer_tokens_msg(
                state.bid_contract.code_hash.clone(),
                &state.bid_contract.address.clone(),
                &bidder.clone(),
                amount,
            )?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    // don't accept a 0 bid
    if amount == Uint128(0) {
        let message = String::from("Bid must be greater than 0");

        let mut resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: None,
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
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

        let mut resp = serde_json::to_string(&HandleAnswer::Bid {
            status: Failure,
            message,
            previous_bid: None,
            amount_bid: None,
            amount_returned: Some(amount),
        })
        .unwrap();

        pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
        return Ok(HandleResponse {
            messages: vec![transfer_tokens_msg(
                state.bid_contract.code_hash.clone(),
                &state.bid_contract.address.clone(),
                &bidder.clone(),
                amount,
            )?],
            log: vec![log("response", resp)],
            data: None,
        });
    }
    let mut return_amount: Option<Uint128> = None;
    let bidder_raw = &deps.api.canonical_address(&bidder.clone())?;
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

                let mut resp = serde_json::to_string(&HandleAnswer::Bid {
                    status: Failure,
                    message,
                    previous_bid: Some(old_bid.amount),
                    amount_bid: None,
                    amount_returned: Some(amount),
                })
                .unwrap();

                pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
                return Ok(HandleResponse {
                    messages: vec![transfer_tokens_msg(
                        state.bid_contract.code_hash.clone(),
                        &state.bid_contract.address.clone(),
                        &bidder.clone(),
                        amount,
                    )?],
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
        cos_msg.push(transfer_tokens_msg(
            state.bid_contract.code_hash.clone(),
            &state.bid_contract.address.clone(),
            &bidder.clone(),
            returned,
        )?);
        message.push_str(". Previously bid tokens have been returned");
    }
    let mut resp = serde_json::to_string(&HandleAnswer::Bid {
        status: Success,
        message,
        previous_bid: None,
        amount_bid: Some(new_bid.amount),
        amount_returned: return_amount,
    })
    .unwrap();

    pad_log_str(RESPONSE_BLOCK_SIZE, &mut resp);
    Ok(HandleResponse {
        messages: cos_msg,
        log: vec![log("response", resp)],
        data: None,
    })
}

// try_retract -- retracts an active bid
//     bidder: &HumanAddr -- address of bidder whose bid should be retracted
// return value: StdResult<HandleResponse>
fn try_retract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    bidder: &HumanAddr,
) -> StdResult<HandleResponse> {
    let mut state = config_read(&deps.storage).load()?;

    let bidder_raw = &deps.api.canonical_address(&bidder.clone())?;
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
            cos_msg.push(transfer_tokens_msg(
                state.bid_contract.code_hash.clone(),
                &state.bid_contract.address,
                &bidder.clone(),
                old_bid.amount,
            )?);
            status = Success;
            sent = Some(old_bid.amount);
            log_msg.push_str("Bid retracted.  Tokens have been returned");
        } else {
            status = Failure;
            sent = None;
            log_msg.push_str(&format!("No active bid for address: {}", bidder.clone()));
        }
    // no active bid found
    } else {
        status = Failure;
        sent = None;
        log_msg.push_str(&format!("No active bid for address: {}", bidder.clone()));
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

// try_finalize -- Closes the auction.  If the sale tokens are fully consigned and if there is at
//                 least one active bid, the sale will finalize with the highest bid (if there is
//                 a tie, the tying bid received first will be the winner). All non-winning bids
//                 will be returned to the bidders.  If there are no bids, the auction will remain
//                 open if the only_if_bids parameter is true.  If the auction closes without
//                 the sale tokens being fully consigned, any consigned tokens will be returned to
//                 the auction creator, and all bids will be returned to the bidders. This can also
//                 be called even after an auction was previously closed.  It will return any bids
//                 or consignment left in the auction's control after it was closed in the event
//                 of some unforeseen error or race-condition.  This should never be needed, but
//                 is provided as a safety precaution
//     only_if_bids: bool -- true if owner does not want to close the auction if there are no bids
//     return_all: bool -- true if called from a return_all execute
// return value: StdResult<HandleResponse>
fn try_finalize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    only_if_bids: bool,
    return_all: bool,
) -> StdResult<HandleResponse> {
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
                cos_msg.push(transfer_tokens_msg(
                    state.bid_contract.code_hash.clone(),
                    &state.bid_contract.address,
                    &state.seller,
                    winning_bid.bid.amount,
                )?);
                cos_msg.push(transfer_tokens_msg(
                    state.sell_contract.code_hash.clone(),
                    &state.sell_contract.address,
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
            cos_msg.push(transfer_tokens_msg(
                state.bid_contract.code_hash.clone(),
                &state.bid_contract.address.clone(),
                &deps.api.human_address(&losing_bid.bidder.clone())?.clone(),
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
        cos_msg.push(transfer_tokens_msg(
            state.sell_contract.code_hash.clone(),
            &state.sell_contract.address,
            &state.seller,
            state.currently_consigned,
        )?);
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

    let mut log_msg = String::new();

    if winning_amount.is_some() {
        log_msg.push_str("Sale finalized.  You have been sent the winning bid tokens");
    } else if amount_returned.is_some() {
        log_msg.push_str("Auction closed.  You have been returned the consigned tokens");
        if !state.tokens_consigned {
            log_msg.push_str(" because you did not consign the full sale amount");
        } else if no_bids {
            log_msg.push_str(" because there were no active bids");
        }
    } else if return_all {
        log_msg.push_str("Outstanding funds have been returned");
    } else {
        log_msg.push_str("Auction has been closed");
    }
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
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::AuctionInfo { .. } => try_query_info(&deps),
    }
}

// try_query_info -- queries all the auction info in the format of msg::QueryAnswer::AuctionInfo
// return value: QueryResult
fn try_query_info<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let state = config_read(&deps.storage).load()?;

    let queryreq = Binary(Vec::from("{\"token_info\":{}}"));

    // get sell token info
    let sell_token_info: TokenQuery = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: state.sell_contract.address.clone(),
            callback_code_hash: state.sell_contract.code_hash.clone(),
            msg: queryreq.clone(),
        }))
        .map_err(|err| {
            StdError::generic_err(format!(
                "{{\"error\": \"Error getting sell token {} info: {}\"}}",
                state.sell_contract.address, err
            ))
        })?;

    // get bid token info
    let bid_token_info: TokenQuery = deps
        .querier
        .query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: state.bid_contract.address.clone(),
            callback_code_hash: state.bid_contract.code_hash.clone(),
            msg: queryreq,
        }))
        .map_err(|err| {
            StdError::generic_err(format!(
                "{{\"error\": \"Error getting bid token {} info: {}\"}}",
                state.bid_contract.address, err
            ))
        })?;

    // build status string
    let mut status = String::new();
    if state.is_completed {
        status.push_str("Closed");
        if !state.bidders.is_empty() || state.currently_consigned > Uint128(0) {
            status.push_str(
                ", but found outstanding balances.  Please run either retract_bid to \
                retrieve your non-winning bid, or return_all to return all outstanding bids/\
                consignment.",
            );
        }
    } else {
        let mut consign = String::new();
        if !state.tokens_consigned {
            consign.push_str(" NOT");
        }
        status.push_str(&format!(
            "Accepting bids: Token(s) to be sold have{} been consigned to the auction",
            consign
        ));
    }

    to_binary(&QueryAnswer::AuctionInfo {
        sell_token: Token {
            contract_address: state.sell_contract.address,
            token_info: sell_token_info.token_info,
        },
        bid_token: Token {
            contract_address: state.bid_contract.address,
            token_info: bid_token_info.token_info,
        },
        sell_amount: state.sell_amount,
        minimum_bid: state.minimum_bid,
        description: state.description,
        auction_address: state.auction_addr,
        status,
    })
}

// transfer_tokens_msg - generates a callback msg to transfer tokens
//    code_hash: String -- code hash of the contract of the token you want to send
//    contract_addr: &HumanAddr -- contract address of the token you want to send
//    recipient: &HumanAddr -- address you are sending to
//    amount: Uint128 -- amount you are sending
//
// return value: StdResult<CosmosMsg>
fn transfer_tokens_msg(
    code_hash: String,
    contract_addr: &HumanAddr,
    recipient: &HumanAddr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let transfer_msg = TransferMsg::new(recipient.clone(), amount);
    transfer_msg.into_cosmos_msg(code_hash, contract_addr.clone())
}
