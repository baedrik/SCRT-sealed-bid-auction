# Sealed Bid Auction

Be sure to read the [WALKTHROUGH.md](https://github.com/baedrik/SCRT-sealed-bid-auction/blob/master/WALKTHROUGH.md) if you are using this contract as a template to build your own secret contract...

And read [CALLING_OTHER_CONTRACTS.md](https://github.com/baedrik/SCRT-sealed-bid-auction/blob/master/CALLING_OTHER_CONTRACTS.md) for an explanation and usage examples of how to allow your contract to call other contracts.

## Notice
There is an [update](https://github.com/baedrik/secret-auction-factory) to this sealed-bid auction contract.  The new implementation incorporates the use of a factory contract to allow someone to view only the active auctions or only the closed auctions.  They may also view only the auctions that they have created, the auctions in which they have active bids, and the auctions they have won if they have created a viewing key with the factory contract.  If they have created a viewing key, the factory will automatically set the viewing key with every active auction they have bids with in order for the bidder to view their bid information directly from the auction.

Although this original auction contract will no longer be updated, the repo will remain available so that people may use it as an example for writing secret contracts.

## And now back to the show...
This is a contract that implements a sealed bid auction where the bid and sold tokens are SNIP-20 compliant.  Technically they don't have to be fully compliant.  They just need to follow SNIP-20 specs for the Send function (as well as the RegisterReceive function, which is needed to set up the Send/Receive functionality).  SNIP-20 spec requires that Send should callback a Receive function with the following msg format:
```sh
pub struct Snip20ReceiveMsg {
    pub sender: HumanAddr,
    pub from: HumanAddr,
    pub amount: Uint128,
    pub msg: Option<Binary>,
}
```

where "sender" is the address that is sending the tokens, "from" is the owner of the tokens, "amount" is how many tokens were sent, and "msg" is optionally defined by the user and is just passed along from the original SendMsg.

The bid/sell tokens should also follow the SNIP-20 format for TokenInfo queries.  Until that format is finalized, the following format is assumed (based on current secretSCRT implementation):
```sh
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Option<Uint128>,
}
```

The next iteration of the auction will also include the ability to accept a ReceiveNft call, so that it can be used with SNIP-721 non-fungible tokens.  The NFT version will likely include having optional TokenID fields when creating the auction, to specify that a specific NFT is up for sale.  The creator could also specify that only a specific NFT Token ID will be accepted as a bid.  This technically would no longer be an auction if only one Token ID will be accepted for a bid, but it would allow the ability to facilitate the trading of 1 specific NFT for another in a trustless manner.  It could also allow someone to essentially post an offer to buy a specific NFT for his asking price (of either a specific NFT or an amount of SNIP-20 tokens)

## Running On Holodeck-2 Testnet
If you want to run sealed-bid auctions on Holodeck-2 Testnet, I have stored the snip20-reference-impl. Its code ID is 1. You can create a new token with
```sh
secretcli tx compute instantiate 1 '{"name": "*token_name*","admin": "*address_with_admin_privileges*", "symbol": "*token_symbol*", "decimals": *decimal_places*, "initial_balances": [{"address": "*address1*", "amount": "*amount_for_address1*"}], "prng_seed": "*base64_encoded_string*","config": {"public_total_supply": *true_or_false*}}' --from *your_key_alias_or_addr* --label *name_for_the_contract* -y
```
You may include as many address/amount pairs as you like in the initial_balances field.

You will want to create a token for sale as well as a token to bid in, because the auction will currently not allow the sale token and bid token to be the same (there is no reason to swap different amounts of the same fungible token).  When the SNIP-721 spec is more fleshed out, this will probably be changed to allow for the exchanging of different NFT token IDs regardless of whether they are part of the same NFT contract or not.  Once you have created your test sale and bid tokens, you are ready to create an auction.  I have also stored this sealed-bid auction contract on Holodeck-2 testnet.  Its code ID is 102.

I have created a bash script file (named auction.sh) to make it easier to use the auction contract.  It requires that you have secretcli and jq installed.  
You can install jq with
```sh
sudo apt-get install jq
```
The script is expecting the auction contract to have code ID 17 (which is the code ID on mainnet), but you can change that on the first line if you store a new version of the contract, or if you want to use it on holodeck.

This script was primarily written as a quick-and-dirty helper for my own testing purposes. If you intend to create a production UI for the contract, when placing a bid, you should follow the example of the script and use the optional padding field when calling a token contract's Send.  You will want the number of digits of the send amount + the number of characters in the padding field to be a constant number (I use 40 characters, because the maximum number of digits of Uint128 is 39, and I always want at least one blank in padding).  This way you do not leak information about the number of digits of the bid.  You do not have to do this for consign, because the consignment amount is public (it is the amount of tokens for sale in the auction), but it will not cause any problems if you also pad the consignment call to Send if the same function is used to Send the tokens for both placing bids, and consigning tokens.

## Creating a new auction
You can create a new auction with
```sh
secretcli tx compute instantiate 102 '{"sell_contract": {"code_hash": "*sale_tokens_code_hash*", "address": "*sale_tokens_contract_address*"}, "bid_contract": {"code_hash": "*bid_tokens_code_hash*", "address": "*bid_tokens_contract_address*"}, "sell_amount": "*amount_being_sold_in_smallest_denomination_of_sale_token*", "minimum_bid": "*minimum_accepted_bid_in_smallest_denomination_of_bid_token*", "description": "*optional_text_description*"}' --from *your_key_alias_or_addr* --label *name_for_the_auction* --gas 300000 -y
```
You can find a contract's code hash with
```sh
secretcli q compute contract-hash *contract_address*
```
Copy it without the 0x prefix and surround it with quotes in the instantiate command.

The description field is optional.  It will accept a free-form text string (best to avoid using double-quotes).  One possible use would be to list the approximate date that you plan to finalize the auction.  In a sealed bid auction, a pre-defined end date is not necessary.  It is necessary in an open ascending bid auction because bidders need to know when the auction will close so that they can monitor if they are winning and bid higher if they are not.  Because in a sealed bid auction, no one knows if they are the highest bidder until after the auction ends, the bidder has no further actions after placing his bid.  For this reason, the auction owner can finalize the auction at any time.  If at any point a bidder no longer wants to wait for the owner to finalize the auction, he can retract his bid and have his bid tokens returned.  For this reason, it might benefit the auction owner to give an approximate end date in the description so that his highest bid doesn't get retracted before he decides to close the auction.  If user consensus would like to have an end date implemented, in which no bids will be accepted after such time, the owner can not finalize the auction before the end date, and afterwards, anyone can close the auction, it can be included.

The auction will not allow a sale amount of 0

The auction will not currently allow the sale contract address to be the same as the bid contract address, because there is no reason to swap different amounts of the same fungible token.  When the SNIP-721 spec is more fleshed out, this will probably be changed to allow for the exchanging of different NFT token IDs regardless of whether they are part of the same NFT contract or not.

## Viewing the Auction Information
You can view the sell and bid token information, the amount being sold, the minimum bid, the description if present, the auction contract address, and the status of the auction with
```sh
secretcli q compute query *auction_contract_address* '{"auction_info":{}}'
```
There is a lot of info there, so it is best viewed piping it through jq
```sh
secretcli q compute query *auction_contract_address* '{"auction_info":{}}'|jq
```
Status will either be "Closed" if the auction is over, or it will be "Accepting bids".  If the auction is closed, it will also display the winning bid if there was one.  If the auction is accepting bids, auction_info will also tell you if the auction owner has consigned the tokens to be sold to the auction.  You may want to wait until the owner consigns the tokens before you bid, but there is no risk in doing it earlier.  At any time before the auction closes, you can retract your bid to have your tokens returned to you.  But if you wait until the owner consigns his tokens, you can be more sure the owner is likely to finalize the auction, because once the tokens to be sold are consigned to the auction, he can not get his orignal tokens or the bid tokens until he finalizes the auction.  The original consigned tokens will only be returned if there are no active bids (either no bids were placed meeting the minimum asking price or all qualifying bids have been retracted).  Otherwise, the highest bid placed at the time of closure will be accepted and the swap will take place.

If the auction is closed, it will display if there are any outstanding funds still residing in the auction account.  This should never happen, but if it does for some unforeseen reason, it will remind the user to either use retract\_bid to have their bid tokens returned (if they haven't already been returned), or use return\_all to return all the funds still held by the auction.  Return\_all can only be called after the auction has closed.

## Consigning Tokens To Be Sold
To consign the tokens to be sold, the owner should Send the tokens to the contract address with
```sh
secretcli tx compute execute *sale_tokens_contract_address* '{"send": {"recipient": "*auction_contract_address*", "amount": "*amount_being_sold_in_smallest_denomination_of_sell_token*"}}' --from *your_key_alias_or_addr* --gas 500000 -y
```
It will only accept consignment from the address that created the auction.  Any other address trying to consign tokens will have them immediately returned.  You can consign an amount smaller than the total amount to be sold, but the auction will not be displayed as fully consigned until you have sent the full amount.  You may consign the total amount in multiple Send transactions if desired, and any tokens you send in excess of the sale amount will be returned to you.  If the auction has been closed, any tokens you send for consignment will be immediately returned, and the auction will remain closed.

## Placing Bids
To place a bid, the bidder should Send the tokens to the contract address with
```sh
secretcli tx compute execute *bid_tokens_contract_address* '{"send": {"recipient": "*auction_contract_address*", "amount": "*bid_amount_in_smallest_denomination_of_bidding_token*"}}' --from *your_key_alias_or_addr* --gas 500000 -y
```
The tokens bid will be placed in escrow until the auction has concluded or you call retract\_bid to retract your bid and have all tokens returned.  You may retract your bid at any time before the auction ends. You may only have one active bid at a time.  If you place more than one bid, the smallest bid will be returned to you, because obviously that bid will lose to your new bid if they both stayed active.  If you bid the same amount as your previous bid, it will retain your original bid's timestamp, because, in the event of ties, the bid placed earlier is deemed the winner.  If you place a bid that is less than the minimum bid, those tokens will be immediately returned to you.  Also, if you place a bid after the auction has closed, those tokens will be immediately returned.

The auction will not allow a bid of 0.

It is recommended that the UI designed to send a bid use the optional "padding" field when calling the bid token contract's Send function.  You should make the number of digits of the bid amount + the number of characters in the "padding" field a constant.  That way the size of the Send does not leak information about the size of the bid.  The helper auction.sh ensures that the number of digits of the bid + the number of spaces sent in the "padding" field always adds up to 40.  Any other UI (or a cmdline call) would do best to implement something similar.

## View Your Active Bid
You may view your current active bid amount and the time the bid was placed with
```sh
secretcli tx compute execute *auction_contract_address* '{"view_bid": {}}' --from *your_key_alias_or_addr* --gas 200000 -y
```
You must use the same address that you did the Send transaction with to view the bid.  Based on feedback, viewing keys could be implemented if users wanted to be able to see the bid through a query in addition to viewing through an execute.

## Retract Your Active Bid
You may retract your current active bid with
```sh
secretcli tx compute execute *auction_contract_address* '{"retract_bid": {}}' --from *your_key_alias_or_addr* --gas 300000 -y
```
You may retract your bid at any time before the auction closes to both retract your bid and to return your tokens.  In the unlikely event that your tokens were not returned automatically when the auction ended, you may call retract_bid after the auction closed to return them manually.

## Finalizing the Auction Sale
The auction creator may close an auction with
```sh
secretcli tx compute execute *auction_contract_address* '{"finalize": {"only_if_bids": *true_or_false*}}' --from *your_key_alias_or_addr* --gas 2000000 -y
```
Only the auction creator can finalize an auction.  The boolean only\_if\_bids parameter is used to prevent the auction from closing if there are no active bids.  If there are no active bids, but only\_if\_bids was set to false, then the auction will be closed, and all consigned tokens will be returned to the auction creator. 
If the auction is closed before the auction creator has consigned all the tokens for sale, any tokens consigned will be returned to the auction creator, and any active bids will be returned to the bidders.  If all the sale tokens have been consigned, and there is at least one active bid, the highest bid will be accepted (if tied, the tying bid placed earlier will be accepted).  The auction will then swap the tokens between the auction creator and the highest bidder, and return all the non-winning bids to their respective bidders.

## Returning Funds In The Event Of Error
In the unlikely event of some unforeseen error that results in funds being held by an auction after it has closed, anyone may run
```sh
secretcli tx compute execute *auction_contract_address* '{"return_all": {}}' --from *your_key_alias_or_addr* --gas 2000000 -y
```
Return_all may only be called after an auction is closed.  Auction\_info will indicate whether any funds are still held by a closed auction.  Even if return\_all is not called, bidders who have not received their bids back can still call retract\_bid to have their bids returned.

## Notes for UI builders
It is recommended that the UI designed to send a bid use the optional "padding" field when calling the bid token contract's Send function.  You will want the number of digits of the send amount + the number of characters in the padding field to be a constant number (I use 40 characters, because the maximum number of digits of Uint128 is 39, and I always want at least one blank in padding).  That way the size of the Send does not leak information about the size of the bid.

Also, you should be aware that responses from bidding and consigning (functions that are called indirectly when doing a Send tx with a token contract) are sent in the log attributes.  This is because when one contract calls another contract, only logs (not the data field) are forwarded back to the user.  On the other hand, any time you call the auction contract directly, the response will be sent in the data field, which is the preferred method of returning json responses.