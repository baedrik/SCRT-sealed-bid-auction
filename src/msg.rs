use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ContractInfo;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, HumanAddr, StdResult, Uint128, WasmMsg};

// Instantiating an auction requires:
//     sell_contract: ContractInfo -- code hash and address of SNIP-20 contract of token for sale
//     bid_contract: ContractInfo -- code hash and address of SNIP-20 contract of bid token
//     sell_amount: Uint128 -- the amount you are selling
//     minimum_bid: Uint128 -- minimum bid you will accept
// optional:
//     description: String -- free-form description of the auction (best to avoid double quotes).
//                            As an example it could be the date the owner will likely finalize the
//                            auction, or a list of other auctions for the same token, etc...
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InitMsg {
    CreateAuction {
        sell_contract: ContractInfo,
        bid_contract: ContractInfo,
        sell_amount: Uint128,
        minimum_bid: Uint128,
        #[serde(default)]
        description: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // Receive gets called by the token contracts of the auction.  If it came from the sale token,
    // it will consign the sent tokens.  If it came from the bid token, it will place a bid.  If
    // any other address tries to call this, it will give an error message that the calling address
    // is not a token in the auction.
    //     sender: HumanAddr -- address of the token contract calling Receive
    //     from: HumanAddr -- owner of the tokens sent to the auction
    //     amount: Uint128 -- amount of tokens sent
    //     msg: Option<Binary> -- not needed or used by this contract
    Receive {
        sender: HumanAddr,
        from: HumanAddr,
        amount: Uint128,
        #[serde(default)]
        msg: Option<Binary>,
    },

    // RetractBid will retract any active bid the calling address has made and return the tokens that
    // are held in escrow
    RetractBid {},

    // ViewBid will display the amount of the active bid made by the calling address and time the bid
    // was placed
    ViewBid {},

    // Finalize will close the auction
    //     only_if_bids: bool -- true if auction creator wants to keep the auction open if there are no
    //                           active bids
    Finalize {
        only_if_bids: bool,
    },

    // If the auction holds any funds after it has closed (should never happen), this will return those
    // funds to their owners.  Should never be needed, but included in case of unforeseen error
    ReturnAll {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Returns the contract address and TokenInfo of the token(s) to be sold, the contract address
    // and TokenInfo of the token(s) that would be accepted as bids, the amount of token(s) to be
    // sold, the minimum bid that will be accepted, an optional description of the auction, the
    // contract address of the auction, and the status of the auction (Accepting bids: Tokens to be
    // sold have(not) been consigned; Closed (will also state if there are outstanding funds after
    // auction closure))
    AuctionInfo {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    AuctionInfo {
        sell_token: Token,
        bid_token: Token,
        sell_amount: Uint128,
        minimum_bid: Uint128,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        auction_address: HumanAddr,
        status: String,
    },
}

// Wraps the return of a token_info query on the SNIP-20 contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenQuery {
    pub token_info: TokenInfo,
}

// structure of token related data used in the auction_info query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Token {
    pub contract_address: HumanAddr,
    pub token_info: TokenInfo,
}

// Speculative format for SNIP-20 Token_info calls.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

// Responses from handle functions.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    Consign {
        status: ResponseStatus,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_consigned: Option<Uint128>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_needed: Option<Uint128>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    Bid {
        status: ResponseStatus,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_bid: Option<Uint128>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_bid: Option<Uint128>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    CloseAuction {
        status: ResponseStatus,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        winning_bid: Option<Uint128>,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    RetractBid {
        status: ResponseStatus,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    Status {
        status: ResponseStatus,
        message: String,
    },
}

// used to serialize the message to transfer tokens
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TransferMsg {
    pub recipient: HumanAddr,
    pub amount: Uint128,
    pub padding: Option<String>,
}

impl TransferMsg {
    pub fn new(recipient: HumanAddr, amount: Uint128) -> Self {
        let padding = Some(" ".repeat(40 - amount.to_string().chars().count()));
        Self {
            recipient,
            amount,
            padding,
        }
    }
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = TransferHandleMsg::Transfer(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(
        self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            msg,
            callback_code_hash,
            contract_addr,
            send: vec![],
        };
        Ok(execute.into())
    }
}
// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum TransferHandleMsg {
    Transfer(TransferMsg),
}

// used to serialize the message to register a receive function with a SNIP-20 contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RegisterMsg {
    pub code_hash: String,
}
impl RegisterMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = RegisterHandleMsg::RegisterReceive(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(
        self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            msg,
            callback_code_hash,
            contract_addr,
            send: vec![],
        };
        Ok(execute.into())
    }
}
// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum RegisterHandleMsg {
    RegisterReceive(RegisterMsg),
}

// space_pad -- pad a Vec<u8> with blanks at the end to length of multiples of block_size
pub fn space_pad(block_size: usize, message: &mut Vec<u8>) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}

// pad_log_str -- pad a String with blanks so that it has length of block_size
pub fn pad_log_str(block_size: usize, response: &mut String) {
    response.push_str(&(" ".repeat(block_size - response.chars().count())));
}
