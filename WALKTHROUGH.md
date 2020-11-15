# Project Structure
The source directory (src/) has these files:

contract.rs  lib.rs  msg.rs  state.rs<br/>
-------------------------------------------
The lib.rs file defines the modules (files) of the contract as well as the entry points of the contract<br/>
-------------------------------------------
The state.rs file defines the State struct, used for storing the contract data, and the Bid struct, used for storing individual bids, keyed by the bidding address.  The state.rs file also defines the functions used to access the contract's storage.<br/>
-------------------------------------------
The msg.rs file is where the InitMsg parameters are specified (like a constructor).  It also defines all the variants of HandleMsg (functions the contract executes) and their parameters.  In addition, it defines the various QueryMsg and their parameters.  It is also where the structs representing the contract's responses for all of the above are defined.  Because the ContractInfo struct implements functions for sending callback messages and queries to token contracts, that has been included in msg.rs as well.<br/>
-------------------------------------------
The contract.rs file contains contract logic, and implements the contract entry points with the init, handle and query functions.<br/>

# lib.rs
```rust
pub mod contract;
pub mod msg;
pub mod state;
```
Most likely, the only changes you might make to this file is to add another `mod` if you create a new source code file.  The name of the `mod` would be the same name as the file without the ".rs" extension.<br/>
The rest of lib.rs defines the entry points of the contract and can be left as is.

# state.rs
```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

use std::collections::HashSet;

use crate::msg::ContractInfo;

/// state of the auction
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

```
Here you will define the data struct(s) needed to describe the state of your contract.  You will need to update the `use` statements accordingly.  The compiler will give you clearly worded errors if it encounters any undefined types that need to be added in the `use` statements.  If you are using anything defined in another file of the contract, use `use::crate::<filename>`.  In the example above, it is pulling the definition of the ContractInfo struct from the msg module (msg.rs file)
```rust
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
```
This defines the functions to access Singleton storage.  Singleton storage is used for any data that has one occurence for the entire contract (such as the contract's global state).  Each Singleton is given a KEY and is associated with a specified type (in this case the State struct).  It should be noted that you can have multiple Singletons, each with a different KEY, but each KEY will have exactly one set of data associated with it.
```rust
/// bid data
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, JsonSchema)]
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
```
This defines the data type for a Bid and the functions to access the storage used for bids.  Because there are multiple bids in an auction, Singleton storage is not appropriate, so in this case we use Bucket storage.  Bucket storage creates a subspace of storage that will be used with one specific data type.  In this case, the subspace with "bids" PREFIX is associated with the Bid type.  You can have multiple Buckets, where each one will associate a different data type to a separate storage subspace with a unique PREFIX.  Each instance of the data type will be stored using its own key.  In this contract, each bid will be stored/accessed by the address of the bidder as can be seen in the contract.rs file.

# msg.rs
This file defines the structs/enums that represent the messages sent to and received from the contract.
```rust
/// Instantiation message
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// sell contract code hash and address
    pub sell_contract: ContractInfo,
    /// bid contract code hash and address
    pub bid_contract: ContractInfo,
    /// amount of tokens being sold
    pub sell_amount: Uint128,
    /// minimum bid that will be accepted
    pub minimum_bid: Uint128,
    /// Optional free-form description of the auction (best to avoid double quotes). As an example
    /// it could be the date the owner will likely finalize the auction, or a list of other
    /// auctions for the same token, etc...
    #[serde(default)]
    pub description: Option<String>,
}
```
Your contract should have an `InitMsg` struct that defines the paramaters that are required by the instantiation message.  `#[serde(default)]` is used to identify that a parameter is optional, and if it is not present, the default value will be assigned to the field.  In this case it means that the instantiation message MAY include the JSON key "description".  If the "description" key is not provided, the description field will default to None (the default of the Option type).  You can also set different default values if needed.  Please see https://serde.rs/attr-default.html for examples.
```rust
/// Handle messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Receive gets called by the token contracts of the auction.  If it came from the sale token, it
    /// will consign the sent tokens.  If it came from the bid token, it will place a bid.  If any
    /// other address tries to call this, it will give an error message that the calling address is
    /// not a token in the auction.
    Receive {
        /// address of person or contract that sent the tokens that triggered this Receive
        sender: HumanAddr,
        /// address of the owner of the tokens sent to the auction
        from: HumanAddr,
        /// amount of tokens sent
        amount: Uint128,
        /// Optional base64 encoded message sent with the Send call -- not needed or used by this
        /// contract
        #[serde(default)]
        msg: Option<Binary>,
    },

    /// RetractBid will retract any active bid the calling address has made and return the tokens
    /// that are held in escrow
    RetractBid {},

    /// ViewBid will display the amount of the active bid made by the calling address and time the
    /// bid was placed
    ViewBid {},

    /// Finalize will close the auction
    Finalize {
        /// true if auction creator wants to keep the auction open if there are no active bids
        only_if_bids: bool,
    },

    /// If the auction holds any funds after it has closed (should never happen), this will return
    /// those funds to their owners.  Should never be needed, but included in case of unforeseen
    /// error
    ReturnAll {},
}
```
Your contract will define a `HandleMsg` enum to describe all the execute messages (and their required parameters) that your contract implements.  `#[serde(rename_all = "snake_case")]` renames the camel case "RetractBid" to "retract_bid", which is the name that would be used in the "tx compute execute" command.  
```rust
   Receive {
        /// address of person or contract that sent the tokens that triggered this Receive
        sender: HumanAddr,
        /// address of the owner of the tokens sent to the auction
        from: HumanAddr,
        /// amount of tokens sent
        amount: Uint128,
        /// Optional base64 encoded message sent with the Send call -- not needed or used by this
        /// contract
        #[serde(default)]
        msg: Option<Binary>,
    },
```
If your contract will be called by either secretSCRT or a SNIP-20 compliant token contract when it is Sent tokens, you will keep the `HandleMsg::Receive` enum as is.
```rust
/// Queries
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Displays the auction information
    AuctionInfo {},
}
```
Your contract will define a `QueryMsg` enum to define all the query messages (and their required parameters) that your contract accepts.  `#[serde(rename_all = "snake_case")]` renames the camel case "AuctionInfo" to "auction_info", which is the name that would be used in the "query compute query" command.
```rust
/// responses to queries
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// AuctionInfo query response
    AuctionInfo {
        /// sell token address and TokenInfo query response
        sell_token: Token,
        /// bid token address and TokenInfo query response
        bid_token: Token,
        /// amount of tokens being sold
        sell_amount: Uint128,
        /// minimum bid that will be accepted
        minimum_bid: Uint128,
        /// Optional String description of auction
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// address of auction contract
        auction_address: HumanAddr,
        /// status of the auction can be "Accepting bids: Tokens to be sold have(not) been
        /// consigned" or "Closed" (will also state if there are outstanding funds after auction
        /// closure
        status: String,
    },
}

/// token's contract address and TokenInfo response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Token {
    /// contract address of token
    pub contract_address: HumanAddr,
    /// Tokeninfo query response
    pub token_info: TokenInfo,
}
```
The `QueryAnswer` enum is used to define the JSON response for each query message.  `#[serde(skip_serializing_if = "Option::is_none")]` will skip the creation of the following JSON key in the response if that field is an Option that is None.
```rust
/// success or failure response
#[derive(Serialize, Deserialize, Debug, JsonSchema, PartialEq)]
pub enum ResponseStatus {
    Success,
    Failure,
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// response from consign attempt
    Consign {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount consigned
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_consigned: Option<Uint128>,
        /// Optional amount that still needs to be consigned
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_needed: Option<Uint128>,
        /// Optional amount of tokens returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// response from bid attempt
    Bid {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount of previous bid returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_bid: Option<Uint128>,
        /// Optional amount bid
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_bid: Option<Uint128>,
        /// Optional amount of tokens returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// response from closing the auction
    CloseAuction {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount of winning bid
        #[serde(skip_serializing_if = "Option::is_none")]
        winning_bid: Option<Uint128>,
        /// Optional amount of tokens returned form escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// response from attempt to retract bid
    RetractBid {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
        /// Optional amount of tokens returned from escrow
        #[serde(skip_serializing_if = "Option::is_none")]
        amount_returned: Option<Uint128>,
    },
    /// generic status response
    Status {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        message: String,
    },
}
```
The `HandleAnswer` enum is used to define the JSON response for each execute message.  `#[serde(skip_serializing_if = "Option::is_none")]` will skip the creation of the following JSON key in the response if that field is None.
```rust
/// code hash and address of a contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: HumanAddr,
}

impl ContractInfo {
    /// Returns a StdResult<CosmosMsg> used to execute Transfer
    ///
    /// # Arguments
    ///
    /// * `recipient` - reference to address tokens are to be sent to
    /// * `amount` - Uint128 amount of tokens to send
    pub fn transfer_msg(&self, recipient: &HumanAddr, amount: Uint128) -> StdResult<CosmosMsg> {
        transfer_msg(
            recipient,
            amount,
            None,
            BLOCK_SIZE,
            &self.code_hash,
            &self.address,
        )
    }
    /// Returns a StdResult<CosmosMsg> used to execute RegisterReceive
    ///
    /// # Arguments
    ///
    /// * `code_hash` - string slice holding code hash contract to be called when sent tokens
    pub fn register_receive_msg(&self, code_hash: &str) -> StdResult<CosmosMsg> {
        register_receive_msg(code_hash, None, BLOCK_SIZE, &self.code_hash, &self.address)
    }
    /// Returns a StdResult<TokenInfo> from performing TokenInfo query
    ///
    /// # Arguments
    ///
    /// * `deps` - reference to Extern that holds all the external contract dependencies
    pub fn token_info_query<S: Storage, A: Api, Q: Querier>(
        &self,
        deps: &Extern<S, A, Q>,
    ) -> StdResult<TokenInfo> {
        token_info_query(deps, BLOCK_SIZE, &self.code_hash, &self.address)
    }
}
```
This defines a ContractInfo struct to hold the code hash and address of a SNIP20 token contract.  It implements functions to enable you to call the Transfer and RegisterReceive functions of those contracts.  If you want to call another contract's handle functions, you generate the appropriate CosmosMsg and place it into the `messages` Vec of your InitResponse/HandleResponse.  These functions use the snip20 package in https://github.com/enigmampc/secret-toolkit by specifying
```rust
use secret_toolkit::snip20::{register_receive_msg, token_info_query, transfer_msg, TokenInfo};
```
Please look at the Cargo.toml file to see how to define the appropriate dependencies.<br/>
If you need to "roll your own" calls to contracts that do not have toolkit shortcuts, you can do the following:
```rust
use serde::{Serialize};
use cosmwasm_std::{WasmMsg, CosmosMsg, StdResult, Coin, to_binary};
use secret_toolkit::utils::{space_pad};
pub const MSG_BLOCK_SIZE: usize = 256;

#[derive(Serialize)]
pub enum ExampleHandleMsg {
    HandleMsgName {
        some: String,
        data: String,
        fields: String,
    },
}
```
First define an enum that matches the HandleMsg enum of the function you want to call.  I included some example `use`s that you may not already have listed.  Add as needed.
```rust
impl ExampleHandleMsg {
    pub fn to_cosmos_msg(
        &self,
        callback_code_hash: &str,
        contract_addr: &HumanAddr,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, MSG_BLOCK_SIZE);
        let mut send = Vec::new();
        if let Some(amount) = send_amount {
            send.push(Coin {
                amount,
                denom: String::from("uscrt"),
            });
        }
        let execute = WasmMsg::Execute {
            msg,
            contract_addr: contract_addr.clone(),
            callback_code_hash: callback_code_hash.to_string(),
            send,
        };
        Ok(execute.into())
    }
}
```
Then implement a function to turn the HandleMsg enum into a callback message you can place in the `messages` Vec of your InitResponse/HandleResponse.  The code above pads the message to a block size of MSG_BLOCK_SIZE using the space_pad function in the utils package of https://github.com/enigmampc/secret-toolkit.  The code above also enables you to send SCRT along with the HandleMsg if the contract requires it.  If it does not, you can use:
```rust
impl ExampleHandleMsg {
    pub fn to_cosmos_msg(
        &self,
        callback_code_hash: &str,
        contract_addr: &HumanAddr,
    ) -> StdResult<CosmosMsg> {
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, PADDING_BLOCK_SIZE);
        let execute = WasmMsg::Execute {
            msg,
            contract_addr: contract_addr.clone(),
            callback_code_hash: callback_code_hash.to_string(),
            send: vec![],
        };
        Ok(execute.into())
    }
}
```
It is best practice to pad your messages so that their byte size can not be used to glean information about what message was processed.
# contract.rs
```rust
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
```
Your contract will have an `init` function that will be called whenever it is instantiated.  You can access the input parameters as fields of the msg parameter of the type `InitMsg` that you defined in msg.rs.  You can leave this code as is.
```rust
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
```
This is an example of how to save Singleton data using the `config` function defined in state.rs.  First it creates the a new variable of State type.  `env.message.sender` is the address that signed the instantiate message (specified with the "--from" flag).  Then it saves the new `state` variable.
```rust
    Ok(InitResponse {
        messages: vec![
            state
                .sell_contract
                .register_receive_msg(&env.contract_code_hash)?,
            state
                .bid_contract
                .register_receive_msg(&env.contract_code_hash)?,


            ExampleHandleMsg::HandleMsgName {
                some: "a".to_string(), 
                data: "b".to_string(),
                fields: "c".to_string(),
            }
            .to_cosmos_msg(
                &code_hash_of_contract_you_want_to_call,
                &that_contract_adress,
                Some(1000000),
            )?,
        ],
        log: vec![],
    })
}

```
First are two examples of calling the RegisterReceive functions of the sell and bid contracts using the `register_receive_msg` functions implemented by the ContractInfo struct defined in msg.rs.  Then I've included an example of calling the example HandleMsg defined earlier in the walkthrough.  The `messages` field of InitReponse/HandleResponse is a `Vec<CosmosMsg>`.  Anytime you want to call another contract, you push the appropriate CosmosMsg onto that Vec.
```rust
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
}
```
Your contract will have a `handle` function.  It will be called whenever a "tx compute execute" command is performed.  You will change the `match msg` statement to handle each HandleMsg enum you defined in msg.rs.  This is how you direct each HandleMsg to the appropriate function.  `env.message.sender` is the address of who sent this execute message.  If called directly by a user, it will be the "--from" address, and if called by a token contract after a Send or SendFrom message, it will be the contract address of that token.
```rust
     pad_handle_result(response, BLOCK_SIZE)
```
This uses the `pad_handle_result` function in the utils package of https://github.com/enigmampc/secret-toolkit.  This function will pad all LogAttribute key and value Strings, as well as the data field of the HandleResponse portion of a HandleResult (a HandleResult is a `StdResult<HandleResponse>`) to multiples of BLOCK_SIZE.<br/>
<br/>

The `try_view_bid` function is used for the following examples:
```rust
    let state = config_read(&deps.storage).load()?;
```
demonstrates how to load the State data from Singleton storage as defined in state.rs
```rust
    let bidder_raw = &deps.api.canonical_address(bidder)?;
    let bidstore = bids_read(&deps.storage);
```
The first line demonstrates how to convert a HumanAddr into a CanonicalAddr, which is what the auction contract uses as the key to store bids.  It should be noted that `deps.api.canonical_address` only supports addresses with a "secret" prefix.  If you need to work with "secretvaloper" addresses, you should use `bech32` to encode/decode the addresses.<br/>
The second line sets access to the Bid Bucket storage as defined in state.rs
```rust
        let bid = bidstore.may_load(bidder_raw.as_slice())?;
```
This line attempts to read the data stored with the `bidder_raw` key in the `bidstore` storage defined above.  When you use the `may_load` function of storage, it will return an Option wrapping whatever type you specified when defining the Bucket functions in state.rs (in this case, Bid).  If no data is found with that key, the Option will be None.
```rust
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
```rust
    let new_bid = Bid {
        amount,
        timestamp: env.block.time,
    };
```
The creation of new_bid shows an example of getting the current timestamp from `env.block.time`.  The timestamp is the Unix Epoch time in seconds.
```rust
    let mut bid_save = bids(&mut deps.storage);
```
Gets write access to the Bid Bucket storage as defined in state.rs
```rust
    bid_save.save(bidder_raw.as_slice(), &new_bid)?;
```
Shows how to save the new_bid with bidder_raw as the key.
```rust
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
```
Because the number of log key-value pairs are publicly visible, you might want to pass the entire response as a single, constant-length JSON string in a single LogAttribute in order to prevent any data leakage.  This code creates a HandleAnswer::Bid response and serializes it to a JSON String using `serde_json::to_string`.
```rust
        return Ok(HandleResponse {
            messages: vec![state.bid_contract.transfer_msg(bidder, amount)?],
            log: vec![log("response", resp)],
            data: None,
        });
```
This uses the `transfer_msg` function implemented by the ContractInfo struct defined in msg.rs to create a CosmosMsg used to call the Transfer function of the bid token contract, and places it in the `messages` Vec to be executed next.  It then passes the response JSON created above as the "value" String of a LogAttribute using the `log` function.  "response" can be replaced with any String you choose as the JSON "key" of the LogAttribute.  Any time your contract is called by another contract, if you want to pass a response to the user, you must use the `log` field of the InitResponse/HandleResponse, because the `data` field is not passed back to the user.  In this case, `try_bid` is only ever called by the bid token contract.
```rust
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
```
Your contract will have a `query` function.  It will be called whenever a "query compute query" command is performed.  You will change the `match msg` statement to handle each QueryMsg enum you defined in msg.rs.  This is how you direct each QueryMsg to the appropriate function.  This uses the `pad_query_result` function in the utils package of https://github.com/enigmampc/secret-toolkit.  This function will pad all LogAttribute key and value Strings, as well as the data field of the InitResponse portion of a InitResult (an InitResult is a `StdResult<InitResponse>`) to multiples of BLOCK_SIZE.<br/>
<br/>

The `try_query_info` function is used for the following examples:
```rust
   // get sell token info
    let sell_token_info = state.sell_contract.token_info_query(deps)?;
```
This is an example of using the `token_info_query` function implemented by the ContractInfo struct defined in msg.rs to send a TokenInfo query to the sell token contract.  If you needed to "roll your own" query of another contract, you could 
```rust
use core::fmt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use cosmwasm_std::{Extern, Storage, Api, Querier, QueryRequest, WasmQuery};
use secret_toolkit::utils::space_pad;
pub const QUERY_BLOCK_SIZE: usize = 256;

/// QueryAnswer for  QueryOne
#[derive(Deserialize)]
pub struct QueryOneAnswer {
    pub yummy: String,
    pub output: String,
    pub data: String,
}
/// QueryAnswer for QueryTwo
#[derive(Deserialize)]
pub struct QueryTwoAnswer {
    pub even: String,
    pub tastier: String,
    pub output: String,
}
```
First define structs matching the format of the QueryAnswer enums you will be receiving.  I included some `use`s that you may or may not already have.  Add as needed. 
```rust
/// wrapper to deserialize QueryOne response
#[derive(Deserialize)]
pub struct QueryOneAnswerWrapper {
    pub query_one: QueryOneAnswer,
}
/// wrapper to deserialize QueryTwo response
#[derive(Deserialize)]
pub struct QueryTwoAnswerWrapper {
    pub query_two: QueryTwoAnswer,
}
```
Then define some structs to wrap those first structs.  The name of the field MUST match the name of the QueryAnswer enum you are receiving (it is often just the name of the QueryMsg that was called)
```rust
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleQueryMsg {
    QueryOne {
        some: String,
        input: String,
        fields: String,
    },
    QueryTwo {
        more: String,
        input: String,
    },
}
```
Then define an enum matching the QueryMsg formats of the queries you will be calling.
```rust
impl fmt::Display for ExampleQueryMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExampleQueryMsg::QueryOne { .. } => write!(f, "QueryOne"),
            ExampleQueryMsg::QueryTwo { .. } => write!(f, "QueryTwo"),
        }
    }
}
```
You can implement a way to easily print out the name of the QueryMsg being called if you want to customize an error description like below.
```rust
impl ExampleQueryMsg {
    pub fn query<S: Storage, A: Api, Q: Querier, T: DeserializeOwned>(
        &self,
        deps: &Extern<S, A, Q>,
        callback_code_hash: &str,
        contract_addr: &HumanAddr,
    ) -> StdResult<T> {
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, QUERY_BLOCK_SIZE);
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_addr.clone(),
                callback_code_hash: callback_code_hash.to_string(),
                msg,
            }))
            .map_err(|err| {
                StdError::generic_err(format!("Error performing {} query: {}", self, err))
            })
    }
}
```
And finally implement a query function that will be used to send the query.
```rust
   let query_one_resp: QueryOneAnswerWrapper = ExampleQueryMsg::QueryOne {
        some: "a".to_string(),
        input: "b".to_string(),
        fields: "c".to_string(),
    }
    .query(deps, &contract_code_hash, &contract_address)?;
```
This is how you call the query.  As you can see, you are creating an instance of the QueryOne variant of the ExampleQueryMsg enum.  And you are calling the `query` function of that instance, and storing the response in the query_one_resp variable that has QueryAnswerWrapper type.  Don't forget that is the wrapper.  If you want to access the actual QueryAnswer data, you need to 
```rust
    let query_one_answer: QueryOneAnswer = query_one_resp.query_one;
```
access the query_one field of the wrapper.
```rust
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
```
Because QueryResponse is just a Binary, and QueryResult is a `StdResult<QueryResponse>`, all you need to do to create the QueryResult is create an instance of your QueryAnswer enum and pass it to `to_binary` which will serialize it to a JSON string and then convert that to a Binary and return a StdResult wrapping that Binary.
