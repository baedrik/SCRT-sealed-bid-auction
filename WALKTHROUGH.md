# Project Structure
The source directory (src/) has these files:

contract.rs  lib.rs  msg.rs  state.rs
The contract.rs file contains contract logic, contract entry points are init, handle and query functions.
The state.rs file defines the State struct, used for storing the contract data, the only information persisted between multiple contract calls.
The msg.rs file is where the InitMsg parameters are specified (like a constructor), the types of Query (GetCount) and Handle[r] (Increment) messages, and any custom structs for each query response.
The lib.rs file xx?


# lib.rs
```sh
pub mod contract;
pub mod msg;
pub mod state;
```
Most likely, the only changes you might make to this file is to add another `mod` if you create a new source code file.  The name of the `mod` would be the same name as the file without the ".rs" extension.<br/>
The rest of lib.rs defines the entry points of the contract and can be left as is.

# state.rs
```sh
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
// bidders: HashSet<Vec<u8>> -- list of addresses with active bids, can not use CanonicalAddr,
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
```
Here you will define the data struct(s) needed to describe the state of your contract.  You will need to update the `use` statements accordingly.  The compiler will give you clearly worded errors if it encounters any undefined types that need to be added in the `use` statements.
```sh
// functions to save/read the state of the auction
pub static CONFIG_KEY: &[u8] = b"config";

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
```
This defines the functions to access Singleton storage.  Singleton storage is used for any data that has one occurence for the entire contract (such as the contract's global state).  Each Singleton is given a KEY and is associated with a specified type (in this case the State struct).  It should be noted that you can have multiple Singletons, each with a different KEY, but each KEY will have exactly one set of data associated with it.
```sh
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
```
This defines the data type for a Bid and the functions to access the storage used for bids.  Because there are multiple bids in an auction, Singleton storage is not appropriate, so in this case we use Bucket storage.  Bucket storage creates a subspace of storage that will be used with one specific data type.  In this case, the subspace with "bids" PREFIX is associated with the Bid type.  You can have multiple Buckets, where each one will associate a different data type to a separate storage subspace with a unique PREFIX.  Each instance of the data type will be stored using its own key.  In this contract, each bid will be stored/accessed by the address of the bidder as can be seen in the contract.rs file.

# msg.rs
This file defines the structs/enums that represent the messages sent to and received from the contract.
```sh
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub sell_contract: ContractInfo,
    pub bid_contract: ContractInfo,
    pub sell_amount: Uint128,
    pub minimum_bid: Uint128,
    #[serde(default)]
    pub description: Option<String>,
}
```
Your contract should have an `InitMsg` struct that defines the paramaters that are required by the instantiation message.  `#[serde(default)]` is used to identify that a parameter is optional, and if it is not present, the default value will be assigned to the field.  In this case it means that the instantiation message MAY include the JSON key "description".  If the "description" key is not provided, the description field will default to None (the default of the Option type).  You can also set different default values if needed.  Please see https://serde.rs/attr-default.html for examples.
```sh
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
```
Your contract will define a `HandleMsg` enum to describe all the execute messages (and their required parameters) that your contract implements.  `#[serde(rename_all = "snake_case")]` renames the camel case "RetractBid" to "retract_bid", which is the name that would be used in the "tx compute execute" command.  
```sh
    Receive {
        sender: HumanAddr,
        from: HumanAddr,
        amount: Uint128,
        #[serde(default)]
        msg: Option<Binary>,
    },
```
If your contract will be called by either secretSCRT or a SNIP-20 compliant token contract when it is Sent tokens, you will keep the `HandleMsg::Receive` enum as is.
```sh
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
```
Your contract will define a `QueryMsg` enum to define all the query messages (and their required parameters) that your contract accepts.  `#[serde(rename_all = "snake_case")]` renames the camel case "AuctionInfo" to "auction_info", which is the name that would be used in the "query compute query" command.
```sh
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<Uint128>,
}
// structure of token related data used in the auction_info query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Token {
    pub contract_address: HumanAddr,
    pub token_info: TokenInfo,
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
```
The `QueryAnswer` enum is used to define the JSON response for each query message.  `#[serde(skip_serializing_if = "Option::is_none")]` will skip the creation of the following JSON key in the response if that field is an Option that is None.
```sh
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

// Wraps the return of a token_info query on the SNIP-20 contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TokenQuery {
    pub token_info: TokenInfo,
}
```
These structs define the JSON response that will be received when doing a "token_info" query from secretSCRT or a SNIP-20 compliant token.  Please see the `try_query_info` function in contract.rs for an example on how a contract can query another contract.
```sh
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
```
The `HandleAnswer` enum is used to define the JSON response for each execute message.  `#[serde(skip_serializing_if = "Option::is_none")]` will skip the creation of the following JSON key in the response if that field is None.
```sh
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
        let padding = Some(" ".repeat(40 - amount.to_string().len()));
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
```
This code is used to create a CosmosMsg to execute the transfer of tokens of secretSCRT or any SNIP-20 compliant token. The line
```sh
        let padding = Some(" ".repeat(40 - amount.to_string().len()));
```
is used to mask the amount being sent by keeping the length of the transfer message constant.  If you need to obfuscate what message you are executing, you may want to pad every callback message to a set block size replacing the `into_binary` function above with
```sh
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = TransferHandleMsg::Transfer(self);
        let mut data = to_binary(&msg)?;
        space_pad(RESPONSE_BLOCK_SIZE, &mut data.0);
        Ok(data)
    }
```
The `space_pad` function is defined later in msg.rs as
```sh
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
```
and you should define/set RESPONSE_BLOCK_SIZE as needed.  You can also use the space_pad function that has since been added to https://github.com/enigmampc/secret-toolkit, but be aware the order of the input parameters is different in the toolkit version than the version in this file.  Otherwise you can copy this code as is if your contract needs to transfer SNIP-20 compliant tokens.  Please see the usage of the `transfer_tokens_msg` function in contract.rs for an example on how to execute the transfer.
```sh
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
```
This code is used to call the `RegisterReceive` function of a SNIP-20 compliant token.  When a SNIP-20 compliant token contract executes a Send or SendFrom message, if the recipient address is the address of a contract that has registered with the SNIP-20 token contract, it will call the Recieve function of the recipient contract.  The code above can be copied as is, if your contract wants to be called whenever it receives tokens from a Send or SendFrom message.  You can also pad this message to a specified block size in the same way described above for the transfer message, if needed.  Please see the `init` function in contract.rs for an example on how to perform a RegisterReceive callback message.
```sh
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
    response.push_str(&(" ".repeat(block_size - response.len())));
}
```
These functions are used to prevent the length of responses from revealing information about what the contract has executed.  `space_pad` is used to pad HandleResponses to a specified block size, and `pad_log_str` is used to pad a LogAttribute value string to a constant size.  A LogAttribute is a key-value pair of Strings.  If you need to pass the user a response from a Receive message execution (triggered by receiving tokens from a Send or SendFrom message), you need to pass the Receive response through the log field instead of the data field of the HandleResponse.  Because the number of log key-value pairs are publicly visible, you might want to pass the entire response as a single, constant-length JSON string in a single LogAttribute in order to prevent any data leakage.

# contract.rs
```sh
////////////////////////////////////// Init ///////////////////////////////////////
// initialize the auction and register receiving function with token contracts
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
```
Your contract will have an `init` function that will be called whenever it is instantiated.  You can access the input parameters as fields of the msg parameter of the type `InitMsg` that you defined in msg.rs.  You can leave this code as is.
```sh
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
```
This is an example of how to save Singleton data using the `config` function defined in state.rs.  First it creates the a new variable of State type.  `env.message.sender` is the address that signed the instantiate message (specified with the "--from" flag).  Then it saves the new `state` variable.
```sh
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
```
This is an example of how to do a RegisterReceive callback message using the `RegisterMsg` struct/functions defined in msg.rs.  The "messages" field of InitReponse/HandleResponse is a `Vec<CosmosMsg>`.  Anytime you want to call another contract, you push the appropriate CosmosMsg onto that Vec.  In this case, we need to register with both the sell and bid contracts, so we are creating/pushing two messages.
```sh
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
```
Your contract will have a `handle` function.  It will be called whenever a "tx compute execute" command is performed.  You will change the `match msg` statement to handle each HandleMsg enum you defined in msg.rs.  This is how you direct each HandleMsg to the appropriate function.  `env.message.sender` is the address of who sent this execute message.  If called directly by a user, it will be the "--from" address, and if called by a token contract after a Send or SendFrom message, it will be the contract address of that token.
```sh
    response.map(|mut response| {
        response.data = response.data.map(|mut data| {
            space_pad(RESPONSE_BLOCK_SIZE, &mut data.0);
            data
        });
        response
    })
}
```
This is used to pad all HandleResponse data fields to a specified block size.  A `pad_handle_result` function has since been added to https://github.com/enigmampc/secret-toolkit which you can use instead of this code.<br/>
<br/>
The `try_view_bid` function is used for the following examples:
```sh
    let state = config_read(&deps.storage).load()?;
```
demonstrates how to load the State data from Singleton storage as defined in state.rs
```sh
    let bidder_raw = &deps.api.canonical_address(&bidder.clone())?;
    let bidstore = bids_read(&deps.storage);
```
The first line demonstrates how to convert a HumanAddr into a CanonicalAddr, which is what the auction contract uses as the key to store bids.  It should be noted that https://github.com/enigmampc/SecretNetwork/blob/master/docs/dev/developing-secret-contracts.md states that it is better to use `bech32` instead of `deps.api.canonical_address` because `deps.api.canonical_address` only supports addresses with a "secret" prefix, not "secretvaloper" addresses.  In this case, only "secret" addresses will be placing bids, so the contract was not updated later to use `bech32`<br/>
The second line sets access to the Bid Bucket storage as defined in state.rs
```sh
        let bid = bidstore.may_load(bidder_raw.as_slice())?;
```
This line attempts to read the data stored with the `bidder_raw` key in the `bidstore` storage defined above.  When you use the `may_load` function of storage, it will return an Option wrapping whatever type you specified when defining the Bucket functions in state.rs (in this case, Bid).  If no data is found with that key, the Option will be None.
```sh
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
```
This is an example of returning the appropriate HandleResponse after execution.  In this case, because it was not necessary to call another contract, the `messages` Vec is empty, and the `log` Vec is empty because we are passing the results in the `data` field.  Here we see an instance of `HandleAnswer::Bid` (defined in msg.rs) being created and passed to `to_binary`.  `to_binary` serializes the `HandleAnswer::Bid` into a JSON string and then converts it into a Binary.  Because the `data` field is defined as `Option<Binary>`, you then need to wrap it with `Some`.<br/>
<br/>
The `try_bid` function is used for the following examples:
```sh
    let new_bid = Bid {
        amount,
        timestamp: env.block.time,
    };
```
The creation of new_bid shows an example of getting the current timestamp from `env.block.time`
```sh
    let mut bid_save = bids(&mut deps.storage);
```
Gets write access to the Bid Bucket storage as defined in state.rs
```sh
    bid_save.save(bidder_raw.as_slice(), &new_bid)?;
```
Shows how to save the new_bid with bidder_raw as the key.
```sh
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
```
This code creates a HandleAnswer::Bid response, serializes it to a JSON String using `serde_json::to_string`, and then pads it to a constant length using `pad_log_str` defined in msg.rs
```sh
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
```
This uses the `transfer_tokens_msg` function to create a CosmosMsg used to call the transfer function of the bid token contract (whose code hash and address are stored in the auction state), and places it in the `messages` Vec to be executed next.  It then passes the padded response JSON created above as the "value" String of a LogAttribute using the `log` function.  "response" can be replaced with any String you choose as the JSON "key" of the LogAttribute.  Because `try_bid` is called as a result of the bid token contract calling Receive, the response has to be passed in the `log` field instead of the `data` field of `HandleResponse`.
```sh
fn transfer_tokens_msg(
    code_hash: String,
    contract_addr: &HumanAddr,
    recipient: &HumanAddr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    let transfer_msg = TransferMsg::new(recipient.clone(), amount);
    transfer_msg.into_cosmos_msg(code_hash, contract_addr.clone())
}
```
This function shows how the CosmosMsg used to transfer tokens is created using the structs/functions defined in msg.rs
```sh
/////////////////////////////////////// Query /////////////////////////////////////
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::AuctionInfo { .. } => try_query_info(&deps),
    }
}
```
Your contract will have a `query` function.  It will be called whenever a "query compute query" command is performed.  You will change the `match msg` statement to handle each QueryMsg enum you defined in msg.rs.  This is how you direct each QueryMsg to the appropriate function.<br/>
<br/>
The `try_query_info` function is used for the following examples:
```sh
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
```
This code shows how to query another contract.  In this case, it is calling the `token_info` query of the sell token contract whose address and code hash is stored in the auction state.  It expects the response to match the format specified by the TokenQuery and TokenInfo structs defined in msg.rs
```sh
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
```
Because QueryResponse is just a Binary, all you need to do to create the QueryResponse is create an instance of your QueryAnswer enum and pass it to `to_binary` which will serialize it to a JSON string and then convert that to a Binary.
