# Project Structure
The source directory (src/) has these files:

contract.rs  lib.rs  msg.rs  state.rs<br/>
-------------------------------------------
The lib.rs file defines the modules (files) of the contract as well as the entry points of the contract<br/>
<br/>
The state.rs file defines the State struct, used for storing the contract data, and the Bid struct, used for storing individual bids, keyed by the bidding address.  The state.rs file also defines the functions used to access the contract's storage.<br/>
<br/>
The msg.rs file is where the InitMsg parameters are specified (like a constructor).  It also defines all the variants of HandleMsg (functions the contract executes) and their parameters.  In addition, it defines the variants of QueryMsg and their parameters.  It is also where the structs/enums representing the contract's responses for all of the above are defined.  Because the ContractInfo struct implements functions for sending callback messages and queries to token contracts, that has been included in msg.rs as well.<br/>
<br/>
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

```
Here you will define the data struct(s) needed to describe the state of your contract.  You will need to update the `use` statements accordingly.  The compiler will give you clearly worded errors if it encounters any undefined types that need to be added in the `use` statements.  If you are using anything defined in another file of the contract, use `use::crate::<filename>`.  In the example above, it is pulling the definition of the ContractInfo struct from the msg module (msg.rs file).  Please see the Cargo.toml file for examples on how to define the appropriate dependencies that these `use` statements rely upon.<br/>
If you need 128-bit unsigned integers, you should use u128 for any data structs that will be stored, and Uint128 for any data structs that will be used as JSON input/output.  This is because serde_json_wasm can not serialize u128, so Uint128 was created as a helper to serialize a u128 as a String.  Because Uint128 serializes as a String, if you commit it to storage, it will have a variable length, depending on the number of digits.  This can be a data leak because storing more bytes incurs a greater gas fee.  You will likely need to convert between the two types as you go back and forth from storage and the contract's input/output.  You can convert a Uint128 to a u128 by using the `u128()` method of Uint128, and you can create a Uint128 from a u128 with `Uint128(some_u128_variable)`.<br/>
It is also best practice to avoid using the cosmwasm storage types Singleton, Typed, and Bucket because these types use serde_json_wasm to serialize the data before it is stored.  Serializing numbers and Options with serde_json_wasm will create variable length serialization, depending on their values, which can be a data leak as described above.  Instead you should use bincode2 for storage serialization.  It not only serializes numbers and Options with a constant length, but it is also more efficient.
```rust
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
```
This defines the data type for a Bid as well as the functions used to access storage.  The difference between `load` and `may_load` is `load` will throw a StdError::NotFound if there is no item stored with the specified key, while `may_load` will return an Ok result of None.  These functions use Bincode2 from the serialization package of https://github.com/enigmampc/secret-toolkit to serialize the data, so they are more secure than the standard cosmwasm storage types Singleton, Typed, and Bucket.  Bincode2 in the toolkit is a helpful wrapper for bincode2 that maps errors so that they also specify the type that was attempting to be (de)serialized.  If you plan to use Bincode2 from the toolkit, 
```rust
use secret_toolkit::serialization::{Bincode2, Serde};
```
you will need to include both Serde and Bincode2 in your `use` statement.  These functions can be copied as-is for use with any types.

# msg.rs
This file defines the structs/enums that represent the messages sent to and received from the contract.
```rust
/// Instantiation message
#[derive(Serialize, Deserialize, JsonSchema)]
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
#[derive(Serialize, Deserialize, JsonSchema)]
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
Your contract will define a `HandleMsg` enum to describe all the execute messages (and their required parameters) that your contract implements.  `#[serde(rename_all = "snake_case")]` renames camel case, i.e. "RetractBid" becomes "retract_bid", which is the name that would be used in the "tx compute execute" command.  
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
If your contract will be called by any SNIP-20 compliant token contract, including secretSCRT, when it is Sent tokens, you will keep the `HandleMsg::Receive` enum as is.
```rust
/// Queries
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Displays the auction information
    AuctionInfo {},
}
```
Your contract will define a `QueryMsg` enum to define all the query messages (and their required parameters) that your contract accepts.  `#[serde(rename_all = "snake_case")]` renames camel case, i.e. "AuctionInfo" becomes "auction_info", which is the name that would be used in the "query compute query" command.
```rust
/// responses to queries
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
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
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
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
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
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
#[derive(Serialize, Deserialize, JsonSchema)]
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
    /// * `recipient` - address tokens are to be sent to
    /// * `amount` - Uint128 amount of tokens to send
    pub fn transfer_msg(&self, recipient: HumanAddr, amount: Uint128) -> StdResult<CosmosMsg> {
        transfer_msg(
            recipient,
            amount,
            None,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }

    /// Returns a StdResult<CosmosMsg> used to execute RegisterReceive
    ///
    /// # Arguments
    ///
    /// * `code_hash` - String holding code hash contract to be called when sent tokens
    pub fn register_receive_msg(&self, code_hash: String) -> StdResult<CosmosMsg> {
        register_receive_msg(
            code_hash,
            None,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }

    /// Returns a StdResult<TokenInfo> from performing TokenInfo query
    ///
    /// # Arguments
    ///
    /// * `querier` - a reference to the Querier dependency of the querying contract
    pub fn token_info_query<Q: Querier>(&self, querier: &Q) -> StdResult<TokenInfo> {
        token_info_query(
            querier,
            BLOCK_SIZE,
            self.code_hash.clone(),
            self.address.clone(),
        )
    }
}
```
This defines a ContractInfo struct to hold the code hash and address of a SNIP20 token contract.  It implements functions to enable you to call the Transfer and RegisterReceive functions of those contracts.  If you want to call another contract's handle functions, you generate the appropriate CosmosMsg and place it into the `messages` Vec of your InitResponse/HandleResponse.  These functions use the snip20 package in https://github.com/enigmampc/secret-toolkit by specifying
```rust
use secret_toolkit::snip20::{register_receive_msg, token_info_query, transfer_msg, TokenInfo};
```
If you need to "roll your own" calls to contracts that do not have toolkit shortcuts, you can do the following:
```rust
use cosmwasm_std::{WasmMsg, CosmosMsg, StdResult, Coin, to_binary};
use secret_toolkit::utils::{space_pad};
pub const MSG_BLOCK_SIZE: usize = 256;

use example_package::msg::HandleMsg as CallbackHandleMsg;
```
I included some example `use`s that you may not already have listed.  Add as needed.  The last one is the important one.  You first add an "example_package" dependency in the Cargo.toml to point to the repo of the contract you want to call.  Change "example_package" to whatever package name is listed in their Cargo.toml.  The `use` statement above will use all the HandleMsg definitions in the contract you want to call and allow you to refer to them as the CallbackHandleMsg enum.  If their HandleMsg enum is NOT defined in the typical msg.rs file, you will change the `use` statement to include the specific crate it is defined in. Alternatively you could just copy and paste the HandleMsg enum you want to use, and define it in your own contract instead of including the `use` statement.
```rust
...
    HandleMsgName {
        some: String,
        data: String,
        fields: String,
    },
...
}
```
For this example, let's assume the HandleMsg you want to execute is defined as above.<br/>
```rust
trait Callback {
    fn to_cosmos_msg(
        &self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
        send_amount: Option<Uint128>,
    ) -> StdResult<CosmosMsg>;
}

impl Callback for CallbackHandleMsg {
    fn to_cosmos_msg(
        &self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
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
            contract_addr,
            callback_code_hash,
            send,
        };
        Ok(execute.into())
    }
}
```
Then you define a trait that you want to add to the enum you are importing from the other contract, and implement that trait as above.  What you are doing is adding a function to the HandleMsg enum that will return the CosmosMsg you need to add to the `messages` Vec of the InitResponse/HandleResponse.  If you only copy-and-pasted the enum definition, you will not define the trait, and you will change `impl Callback for CallbackHandleMsg {` to `impl CallbackHandleMsg {` (if you named your copy-and-pasted enum CallbackHandleMsg).  The `send_amount` parameter is the amount of uSCRT you want to send to the contract with the HandleMsg.  If the function does not require any SCRT being sent, you will want to call `to_cosmos_msg` with None as the send_amount.<br/>
The code above pads the message to a block size of MSG_BLOCK_SIZE using the space_pad function in the utils package of https://github.com/enigmampc/secret-toolkit.  It is best practice to pad your messages so that their byte size can not be used to glean information about what message was processed.<br/>
Now that we have functions defined to enable calling other contracts, we will see examples below about how to use them.
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
        sell_amount: msg.sell_amount.u128(),
        minimum_bid: msg.minimum_bid.u128(),
        currently_consigned: 0,
        bidders: HashSet::new(),
        is_completed: false,
        tokens_consigned: false,
        description: msg.description,
    };

    save(&mut deps.storage, CONFIG_KEY, &state)?;
```
This is an example of how to save to storage using the function defined in state.rs.  First it creates a new variable of State type.  `env.contract.address` is the address this new contract has been assigned, and `env.message.sender` is the address that signed the instantiate message (specified with the "--from" flag).  Then it saves the new `state` variable.  It uses a key defined earlier in the file as `pub const CONFIG_KEY: &[u8] = b"config";`.
```rust
    Ok(InitResponse {
        messages: vec![
            state
                .sell_contract
                .register_receive_msg(env.contract_code_hash.clone())?,
            state
                .bid_contract
                .register_receive_msg(env.contract_code_hash)?,


            CallbackHandleMsg::HandleMsgName {
                some: "a".to_string(), 
                data: "b".to_string(),
                fields: "c".to_string(),
            }
            .to_cosmos_msg(
                code_hash_of_contract_you_want_to_call,
                that_contracts_address,
                Some(1000000),
            )?,
        ],
        log: vec![],
    })
}

```
Now that the auction contract has done everything it needs to do when instantiated, it is time to call other contracts.  You do this by first creating the InitResponse.  The `messages` field of InitReponse/HandleResponse is a `Vec<CosmosMsg>`.  Anytime you want to call another contract, you push the appropriate CosmosMsg onto that Vec.<br/>
First are two examples of calling the RegisterReceive functions of the sell and bid contracts using the `register_receive_msg` functions implemented by the ContractInfo struct defined in msg.rs.  Then I've included an example of calling the example HandleMsg defined earlier in the walkthrough, which is also sending 1000000uscrt along with the callback message.  
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
        HandleMsg::RetractBid { .. } => try_retract(deps, env.message.sender),
        HandleMsg::Finalize { only_if_bids, .. } => try_finalize(deps, env, only_if_bids, false),
        HandleMsg::ReturnAll { .. } => try_finalize(deps, env, false, true),
        HandleMsg::Receive { from, amount, .. } => try_receive(deps, env, from, amount),
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
    let state: State = load(&deps.storage, CONFIG_KEY)?;
```
This demonstrates how to load the State data from storage using the function defined in state.rs.
```rust
    let bidder_raw = &deps.api.canonical_address(&bidder)?;
```
This demonstrates how to convert a HumanAddr into a CanonicalAddr, which is what the auction contract uses as the key to store bids.  It should be noted that `deps.api.canonical_address` only supports addresses with a "secret" prefix.  If you need to work with "secretvaloper" addresses, you should use `bech32` to encode/decode the addresses.
```rust
        let bid: Option<Bid> = may_load(&deps.storage, bidder_raw.as_slice())?;
```
This line attempts to read the Bid stored with the `bidder_raw` key.  Because it uses the `may_load` function, the result is an `Option`, which may be None if no Bid is found with that key.
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
        amount: amount.u128(),
        timestamp: env.block.time,
    };
```
The creation of new_bid shows an example of getting the current timestamp from `env.block.time`.  The timestamp is the Unix Epoch time in seconds.
```rust
    save(&mut deps.storage, bidder_raw.as_slice(), &new_bid)?;
```
This saves the new bid under the bidder_raw key.  As you can see, the storage functions defined in state.rs can be used with different types.<br/>
Now let's examine an example of returning a HandleResponse after execution which requires calling another contract.  Let's look back at the beginning of the `try_bid` function.
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
Now that the auction contract is done processing the bid (which was rejected in this case because the auction had already closed), it is ready to create the HandleResponse.  In doing so, it also sets up the call to the Transfer method of the bid token contract to return the tokens that were sent to escrow after the auction closed.  It uses the `transfer_msg` function implemented by the ContractInfo struct defined in msg.rs to create the appropriate CosmosMsg and places it in the `messages` Vec to be executed next.  It then passes the response JSON created above as the "value" String of a LogAttribute using the `log` function.  "response" can be replaced with any String you choose as the JSON "key" of the LogAttribute.  Any time your contract is called by another contract, if you want to pass a response to the user, you must use the `log` field of the InitResponse/HandleResponse, because the `data` field is not passed back to the user.  In this case, `try_bid` is only ever called by the bid token contract.
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
Your contract will have a `query` function.  It will be called whenever a "query compute query" command is performed.  You will change the `match msg` statement to handle each QueryMsg enum you defined in msg.rs.  This is how you direct each QueryMsg to the appropriate function.  This uses the `pad_query_result` function in the utils package of https://github.com/enigmampc/secret-toolkit.  This function will pad the QueryResponse portion of a QueryResult (a QueryResult is a `StdResult<QueryResponse>`) to multiples of BLOCK_SIZE.<br/>
<br/>

The `try_query_info` function is used for the following examples:
```rust
   // get sell token info
    let sell_token_info = state.sell_contract.token_info_query(&deps.querier)?;
```
This is an example of using the `token_info_query` function implemented by the ContractInfo struct defined in msg.rs to send a TokenInfo query to the sell token contract.  It returns the TokenInfo type defined in the snip20 package of https://github.com/enigmampc/secret-toolkit.<br/>
If you need to "roll your own" query of another contract, you could 
*****TODO*****Going to re-write this using a Trait like I did for the Callback message implementation above
```rust
use core::fmt;
use serde::{de::DeserializeOwned, Serialize, Deserialize};
use cosmwasm_std::{Querier, QueryRequest, WasmQuery};
use secret_toolkit::utils::space_pad;
pub const QUERY_BLOCK_SIZE: usize = 256;

use example_package::msg::QueryMsg as ExampleQueryMsg;
```
I included some example `use`s that you may not already have listed.  Add as needed.  The last one is the important one.  You first add an "example_package" dependency in the Cargo.toml to point to the repo of the contract you want to call.  Change "example_package" to whatever package name is listed in their Cargo.toml.  The `use` statement above will use all the QueryMsg definitions in the contract you want to call and allow you to refer to them as the ExampleQueryMsg enum.  If their QueryMsg enum is NOT defined in the typical msg.rs file, you will change the `use` statement to include the specific crate it is defined in. Alternatively you could just copy and paste the QueryMsg enum you want to use, and define it in your own contract instead of including the `use` statement.
```rust
/// QueryAnswer for  QueryOne
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QueryOneAnswer {
    pub yummy: String,
    pub output: String,
    pub data: String,
}
/// QueryAnswer for QueryTwo
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QueryTwoAnswer {
    pub even: String,
    pub tastier: String,
    pub output: String,
}
```
Now, define structs matching the format of the QueryAnswer enums you will be receiving.
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
Then define some structs to wrap those first structs.  The name of the field MUST match the snake_case name the other contract gave to the QueryAnswer enum variant(s) you are receiving (it is often just the name of the QueryMsg that was called)
```rust
...
    QueryOne {
        some: String,
        input: String,
        fields: String,
    },
    QueryTwo {
        more: String,
        input: String,
    },
...
}
```
For this example, let's assume the QueryMsg you want to execute is defined as above.
```rust
impl fmt::Display for ExampleQueryMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExampleQueryMsg::QueryOne { .. } => write!(f, "QueryOne"),
            ExampleQueryMsg::QueryTwo { .. } => write!(f, "QueryTwo"),
            // if you are `use`ing the other contract's QueryMsg definitions, you
            // need to add the following line so the `match` statement exhausts 
            // all possible variants of the ExampleQueryMsg enum
            _ => write!(f, "Unspecified QueryMsg"),
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
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<T> {
        let mut msg = to_binary(self)?;
        space_pad(&mut msg.0, QUERY_BLOCK_SIZE);
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                callback_code_hash,
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
        sell_amount: Uint128(state.sell_amount),
        minimum_bid: Uint128(state.minimum_bid),
        description: state.description,
        auction_address: state.auction_addr,
        status,
    })
```
Because QueryResponse is just a Binary, and QueryResult is a `StdResult<QueryResponse>`, all you need to do to create the QueryResult is create an instance of your QueryAnswer enum and pass it to `to_binary` which will serialize it to a JSON string and then convert that to a Binary and return a StdResult wrapping that Binary.
