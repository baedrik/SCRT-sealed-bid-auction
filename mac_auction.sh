#!/usr/local/bin/bash

# Change this to the code ID of the auction contract for whatever chain your secretcli is using
contractcode="17"

# convert denom
convert_denom () {
    local convert=`echo "$1 / 10^$2" | bc -l`
    denom=$(echo $convert | sed '/\./ s/\.\{0,1\}0\{1,\}$//')
}

# text colors
BLUE='\033[1;34m'
GRN='\033[1;32m'
NC='\033[0m'

# display auction info
display_auction_info () {
    auctioninfo=$(secretcli q compute query $auctionaddr '{"auction_info":{}}' --trust-node=true \
                  -o json)
    echo -e "${BLUE}Auction Info:\n"
    echo "Sale Token:"
    echo -e "\tContract Address: ${GRN}$(jq -r '.auction_info.sell_token.contract_address'<<<$auctioninfo)"
    echo -e "\t${BLUE}Name: ${GRN}$(jq -r '.auction_info.sell_token.token_info.name'<<<$auctioninfo)"
    echo -e "\t${BLUE}Symbol: ${GRN}$(jq -r '.auction_info.sell_token.token_info.symbol'<<<$auctioninfo)"
    local saledecimals=$(jq -r '.auction_info.sell_token.token_info.decimals'<<<$auctioninfo)
    echo -e "\t${BLUE}Decimals: ${GRN}$saledecimals"
    echo -e "${BLUE}Bid Token:"
    echo -e "\tContract Address: ${GRN}$(jq -r '.auction_info.bid_token.contract_address'<<<$auctioninfo)"
    echo -e "\t${BLUE}Name: ${GRN}$(jq -r '.auction_info.bid_token.token_info.name'<<<$auctioninfo)"
    echo -e "\t${BLUE}Symbol: ${GRN}$(jq -r '.auction_info.bid_token.token_info.symbol'<<<$auctioninfo)"
    local buydecimals=$(jq -r '.auction_info.bid_token.token_info.decimals'<<<$auctioninfo)
    echo -e "\t${BLUE}Decimals: ${GRN}$buydecimals"
    local saleamount=$(jq -r '.auction_info.sell_amount' <<<$auctioninfo)
    convert_denom $saleamount $saledecimals
    echo -e "${BLUE}Sale Amount: ${GRN}$denom"
    local minimumbid=$(jq -r '.auction_info.minimum_bid' <<<$auctioninfo)
    convert_denom $minimumbid $buydecimals
    echo -e "${BLUE}Minimum Bid: ${GRN}$denom"
    local description=$(jq -r '.auction_info.description' <<<$auctioninfo)
    if [[ "$description" != "null" ]]
    then
        echo -e "${BLUE}Description: ${GRN}$description"
    fi
    echo -e "${BLUE}Auction Address: ${GRN}$(jq -r '.auction_info.auction_address'<<<$auctioninfo)"
    echo -e "${BLUE}Status: ${GRN}$(jq -r '.auction_info.status'<<<$auctioninfo)${NC}"
    local winningbid=$(jq -r '.auction_info.winning_bid' <<<$auctioninfo)
    if [[ "$winningbid" != "null" ]]
    then
        convert_denom $winningbid $buydecimals
        echo -e "${BLUE}Winning Bid: ${GRN}$denom${NC}"
    fi
}

# function to get a contract address, find its code hash and number of decimals
get_contract () {
    read conaddr
    hash=$(secretcli q compute contract-hash "$conaddr" --trust-node=true -o json 2>&1)
    if echo "$hash" | grep ERROR
    then
        goodinp=false
    else
        hash=${hash/0x/}
        local tokeninfo=$(secretcli q compute query "$conaddr" '{"token_info":{}}' \
                          --trust-node=true -o json 2>&1)
        if echo $tokeninfo | grep ERROR
        then
            echo -e "\nAre you sure that is a SNIP-20 token contract address?"
            goodinp=false
        else
            decimals=$(jq -r '.token_info.decimals' <<<"$tokeninfo")
            goodinp=true
        fi
    fi
}

# use to check numerical input
re='^[0-9]*(\.[0-9]+)?$'

# this function will verify input is a numeric in correct form and makes sure there aren't too
# many decimals
get_amount () {
    read inp
    if [[ "$inp" =~ $re ]]
    then
        if [[ "$inp" = *.* ]]
        then
            local dec=${inp#*.}
            local count=${#dec}
            if (( $count > $1 ))
            then
                echo -e "\nYOU ENTERED $count DECIMAL PLACES.\nTOKEN ONLY HAS $1 DECIMALS"
                return
            fi
        fi
        if (( ${#inp} == 0 ))
        then
            echo -e "\nINPUT MUST BE NUMERIC AND CAN NOT END WITH \".\""
            echo -e "EITHER DELETE THE \".\" OR MAKE IT \".0\""
            return
        fi
        amount=`echo "$inp * 10^$1" | bc -l`
        amount=${amount%.*}
        goodinp=true
    else
        echo -e "\nINPUT MUST BE NUMERIC AND CAN NOT END WITH \".\""
        echo -e "EITHER DELETE THE \".\" OR MAKE IT \".0\""
    fi
}

cat << EOF
Just a reminder that you need to have secretcli, jq, and bash v4 or higher installed.
You can install jq with homebrew:   brew install jq
You can update bash with homebrew:  brew install bash

EOF

goodinp=false
while [ $goodinp == false ]
do
    echo -e "\nWhat is the secretcli keys alias of the account you want use?"
    read inp
    addr=$(secretcli q account $(secretcli keys show -a "$inp") --trust-node=true -o json \
           | jq -r '.value.address')
    if echo $addr | grep secret
    then
        addralias=$inp
        goodinp=true
    fi
done

goodinp=false
while [ $goodinp == false ]
do
   cat << EOF


Would you like to:
(c)reate a new auction
(l)ist existing auctions
               (or (q)uit)
EOF
    read inp
    lowcase=$(echo $inp | awk '{print tolower($0)}')
    if [[ "$lowcase" == "create" ]] || [[ "$lowcase" == "c" ]]
    then
        cmd="c"
        goodinp=true
    elif [[ "$lowcase" == "list" ]] || [[ "$lowcase" == "l" ]]
    then
        cmd="l"
        goodinp=true
    elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]
    then
        exit
    fi
done

# list existing auctions
if [[ $cmd == 'l' ]]
then

    echo -e "\n"
    declare -A owners
    declare -A addrs
    auctionlist=$(secretcli q compute list-contract-by-code $contractcode --trust-node=true \
                  -o json)
    if [[ "$auctionlist" == "null" ]]
    then
        echo "There are no auctions.  Try creating one!"
        exit
    fi
    contracttsv=$(jq -r '.[]|[.creator, .label, .address] | @tsv' <<<"$auctionlist")
    while IFS=$'\t' read -r creator label address
    do
        echo $label
        owners+=([$label]=$creator)
        addrs+=([$label]=$address)
    done <<<"$contracttsv"

    # select auction
    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhich auction do you want to view?"
        read inp
        if [ ${owners[${inp}]+_} ]
        then
            auctionlabel=$inp
            goodinp=true
        else
            jq -r '.[].label' <<<"$auctionlist"
            echo "Auction name \"$inp\" not found"
        fi
    done

    auctionowner=${owners[$auctionlabel]}
    auctionaddr=${addrs[$auctionlabel]}
    display_auction_info
    sellcontr=$(jq -r '.[].sell_token.contract_address' <<<"$auctioninfo")
    selldecimals=$(jq -r '.[].sell_token.token_info.decimals' <<<"$auctioninfo")
    bidcontr=$(jq -r '.[].bid_token.contract_address' <<<"$auctioninfo")
    biddecimals=$(jq -r '.[].bid_token.token_info.decimals' <<<"$auctioninfo")
    sellamount=$(jq -r '.[].sell_amount' <<<"$auctioninfo")
    minbid=$(jq -r '.[].minimum_bid' <<<"$auctioninfo")
    auctionstat=$(jq -r '.[].status' <<<"$auctioninfo")

# display options for the owner
    if [[ "$auctionowner" == "$addr" ]] 
    then
      while [ true ]
      do
        goodinp=false
        while [ $goodinp == false ]
        do
           cat << EOF

Would you like to:
(c)onsign tokens to auction escrow
(f)inalize/close the auction
(d)isplay auction info
               (or (q)uit)
EOF
            read inp
            lowcase=$(echo $inp | awk '{print tolower($0)}')
            if [[ "$lowcase" == "consign" ]] || [[ "$lowcase" == "c" ]]
            then
                owncmd="c"
                goodinp=true
            elif [[ "$lowcase" == "finalize" ]] || [[ "$lowcase" == "f" ]]
            then
                owncmd="f"
                goodinp=true
            elif [[ "$lowcase" == "display" ]] || [[ "$lowcase" == "d" ]]
            then
                owncmd="d"
                goodinp=true
            elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]
            then
                exit
            fi
        done
# finalize/close auction
        if [[ $owncmd == 'f' ]]
        then
            goodinp=false
            while [ $goodinp == false ]
            do
                echo "Do you want to keep the auction open if there are currently no active bids?"
                echo "(y)es or (n)o"
                read keepopen
                lowcase=$(echo $keepopen | awk '{print tolower($0)}')
                if [[ "$lowcase" == "yes" ]] || [[ "$lowcase" == "y" ]]
                then
                    onlyif=true
                    goodinp=true
                elif [[ "$lowcase" == "no" ]] || [[ "$lowcase" == "n" ]]
                then
                    onlyif=false
                    goodinp=true
                fi
            done
#
# change --gas amount below if getting out of gas error during finalize/close
#
            resp=$(secretcli tx compute execute $auctionaddr \
                     "{\"finalize\":{\"only_if_bids\":$onlyif}}" --from $addr --gas 2000000 \
                     --broadcast-mode block --trust-node=true -o json -y)
            echo "$resp" | grep "out of gas"
            tx=$(jq -r '.txhash' <<<"$resp")
            decd=$(secretcli q compute tx $tx --trust-node=true -o json)
            fnlresp=$(jq -r '.output_data_as_string' <<<"$decd")
            fnlresp=${fnlresp//\\"/"}
            echo -e "${BLUE}Finalize:\n"
            echo -e "Status: ${GRN}$(jq -r '.close_auction.status'<<<$fnlresp)"
            echo -e "${BLUE}Message: ${GRN}$(jq -r '.close_auction.message'<<<$fnlresp)${NC}"
            fnlwinningbid=$(jq -r '.close_auction.winning_bid'<<<$fnlresp)
            if [[ "$fnlwinningbid" != "null" ]]
            then
                convert_denom $fnlwinningbid $biddecimals
                echo -e "${BLUE}Winning Bid: ${GRN}$denom${NC}"
            fi
            fnlreturned=$(jq -r '.close_auction.amount_returned'<<<$fnlresp)
            if [[ "$fnlreturned" != "null" ]]
            then
                convert_denom $fnlreturned $selldecimals
                echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
            fi
# consign tokens
        elif [[ $owncmd == 'c' ]]
        then
            goodinp=false
            while [ $goodinp == false ]
            do
                echo "How much do you want to consign?"
                echo "Recommend consigning the full sale amount, but you can do it in multiple"
                echo "transactions if you want"
                get_amount $selldecimals
            done
            csnamount=$amount
#
# change --gas amount below if getting out of gas error during consign
#
            resp=$(secretcli tx compute execute $sellcontr "{\"send\":{\"recipient\":\
\"$auctionaddr\",\"amount\":\"$csnamount\"}}" --from $addr --gas 500000 \
                --broadcast-mode block --trust-node=true -o json -y)
            echo "$resp" | grep "out of gas"
            sendtx=$(jq -r '.txhash' <<<"$resp")
            decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
            decdsenderr=$(jq '.output_error' <<<"$decdsend")
            if [[ "$decdsenderr" == "{}" ]]
            then
                padkey=$(printf "%-256s" "response")
                logresp=$(jq -r --arg KEY "$padkey" \
                            '.output_log[0].attributes[]|select(.key==$KEY).value' <<<"$decdsend")
                cleaned=$(echo $logresp | sed 's/\\//g')
                echo -e "${BLUE}Consign:\n"
                echo -e "Status: ${GRN}$(jq -r '.consign.status'<<<$cleaned)"
                echo -e "${BLUE}Message: ${GRN}$(jq -r '.consign.message'<<<$cleaned)${NC}"
                csnamtcsn=$(jq -r '.consign.amount_consigned'<<<$cleaned)
                if [[ "$csnamtcsn" != "null" ]]
                then
                    convert_denom $csnamtcsn $selldecimals
                    echo -e "${BLUE}Amount Consigned: ${GRN}$denom${NC}"
                fi
                csnamtneed=$(jq -r '.consign.amount_needed'<<<$cleaned)
                if [[ "$csnamtneed" != "null" ]]
                then
                    convert_denom $csnamtneed $selldecimals
                    echo -e "${BLUE}Amount Needed: ${GRN}$denom${NC}"
                fi
                csnreturned=$(jq -r '.consign.amount_returned'<<<$cleaned)
                if [[ "$csnreturned" != "null" ]]
                then
                    convert_denom $csnreturned $selldecimals
                    echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
                fi
            else
                echo $decdsenderr
            fi
# display auction info
        elif [[ $owncmd == 'd' ]]
        then
            display_auction_info
        fi
      done
# display options for bidder
    else
      while [ true ]
      do
        goodinp=false
        while [ $goodinp == false ]
        do
           cat << EOF

Would you like to:
(p)lace a new bid
(v)iew an active bid
(r)etract an active bid
(d)isplay auction info
               (or (q)uit)
EOF
            read inp
            lowcase=$(echo $inp | awk '{print tolower($0)}')
            if [[ "$lowcase" == "place" ]] || [[ "$lowcase" == "p" ]]
            then
                bidcmd="p"
                goodinp=true
            elif [[ "$lowcase" == "view" ]] || [[ "$lowcase" == "v" ]]
            then
                bidcmd="v"
                goodinp=true
            elif [[ "$lowcase" == "retract" ]] || [[ "$lowcase" == "r" ]]
            then
                bidcmd="r"
                goodinp=true
            elif [[ "$lowcase" == "display" ]] || [[ "$lowcase" == "d" ]]
            then
                bidcmd="d"
                goodinp=true
            elif [[ "$lowcase" == "quit" ]] || [[ "$lowcase" == "q" ]]
            then
                exit
            fi
        done
# place a bid
        if [[ $bidcmd == 'p' ]]
        then
            goodinp=false
            while [ $goodinp == false ]
            do
                echo "How much do you want to bid?"
                get_amount $biddecimals
            done
            bidamount=$amount

            # need to add padding to hide bid length, Uint128 can have about 40 digits
            bidlen=${#bidamount}
            missing=$(( 40 - bidlen ))
            spaces=$(printf '%*s' $missing)
#
# change --gas amount below if getting out of gas error during place bid
#
            resp=$(secretcli tx compute execute $bidcontr "{\"send\":{\"recipient\":\
\"$auctionaddr\",\"amount\":\"$bidamount\",\"padding\":\"$spaces\"}}"\
                   --from $addr --gas 500000 --broadcast-mode block --trust-node=true -o json -y)

            echo "$resp" | grep "out of gas"
            sendtx=$(jq -r '.txhash' <<<"$resp")
            decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
            decdsenderr=$(jq '.output_error' <<<"$decdsend")
            if [[ "$decdsenderr" == "{}" ]]
            then
                padkey=$(printf "%-256s" "response")
                logresp=$(jq -r --arg KEY "$padkey" \
                            '.output_log[0].attributes[]|select(.key==$KEY).value' <<<"$decdsend")
                cleaned=$(echo $logresp | sed 's/\\//g')
                echo -e "${BLUE}Bid:\n"
                echo -e "Status: ${GRN}$(jq -r '.bid.status'<<<$cleaned)"
                echo -e "${BLUE}Message: ${GRN}$(jq -r '.bid.message'<<<$cleaned)${NC}"
                prevbid=$(jq -r '.bid.previous_bid'<<<$cleaned)
                if [[ "$prevbid" != "null" ]]
                then
                    convert_denom $prevbid $biddecimals
                    echo -e "${BLUE}Previous Bid: ${GRN}$denom${NC}"
                fi
                amountbid=$(jq -r '.bid.amount_bid'<<<$cleaned)
                if [[ "$amountbid" != "null" ]]
                then
                    convert_denom $amountbid $biddecimals
                    echo -e "${BLUE}Amount Bid: ${GRN}$denom${NC}"
                fi
                bidreturned=$(jq -r '.bid.amount_returned'<<<$cleaned)
                if [[ "$bidreturned" != "null" ]]
                then
                    convert_denom $bidreturned $biddecimals
                    echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
                fi
            else
                echo $decdsenderr
            fi
# display auction info
        elif [[ $bidcmd == 'd' ]]
        then
            display_auction_info
# view active bid
        elif [[ $bidcmd == 'v' ]]
        then
#
# change --gas amount below if getting out of gas error during view bid
#
            resp=$(secretcli tx compute execute $auctionaddr '{"view_bid":{}}' --from $addr --gas \
                   200000 --broadcast-mode block --trust-node=true -o json -y)
            echo "$resp" | grep "out of gas"
            tx=$(jq -r '.txhash' <<<"$resp")
            decd=$(secretcli q compute tx $tx --trust-node=true -o json)
            bidresp=$(jq -r '.output_data_as_string' <<<"$decd")
            bidresp=${bidresp//\\"/"}
            echo -e "${BLUE}Bid:\n"
            echo -e "Status: ${GRN}$(jq -r '.bid.status'<<<$bidresp)"
            echo -e "${BLUE}Message: ${GRN}$(jq -r '.bid.message'<<<$bidresp)${NC}"
            amountbid=$(jq -r '.bid.amount_bid'<<<$bidresp)
            if [[ "$amountbid" != "null" ]]
            then
                convert_denom $amountbid $biddecimals
                echo -e "${BLUE}Amount Bid: ${GRN}$denom${NC}"
            fi
 #retract active bid
        elif [[ $bidcmd == 'r' ]]
        then
#
# change --gas amount below if getting out of gas error during retract bid
#
            resp=$(secretcli tx compute execute $auctionaddr '{"retract_bid":{}}' --from $addr \
                   --gas 300000 --broadcast-mode block --trust-node=true -o json -y)
            echo "$resp" | grep "out of gas"
            tx=$(jq -r '.txhash' <<<"$resp")
            decd=$(secretcli q compute tx $tx --trust-node=true -o json)
            bidresp=$(jq -r '.output_data_as_string' <<<"$decd")
            bidresp=${bidresp//\\"/"}
            echo -e "${BLUE}Retract Bid:\n"
            echo -e "Status: ${GRN}$(jq -r '.retract_bid.status'<<<$bidresp)"
            echo -e "${BLUE}Message: ${GRN}$(jq -r '.retract_bid.message'<<<$bidresp)${NC}"
            bidreturned=$(jq -r '.retract_bid.amount_returned'<<<$bidresp)
            if [[ "$bidreturned" != "null" ]]
            then
                convert_denom $bidreturned $biddecimals
                echo -e "${BLUE}Amount Returned: ${GRN}$denom${NC}"
            fi
        fi
      done
    fi
# create new auction
elif [[ $cmd == 'c' ]]
then
    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the contract address of the token you want to sell?"
        get_contract
    done
    selladdr=$conaddr
    sellhash=$hash
    selldecimals=$decimals

    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the contract address of the token you will accept bids in?"
        get_contract
    done
    bidaddr=$conaddr
    bidhash=$hash
    biddecimals=$decimals

    goodinp=false
    while [ $goodinp == false ]
    do
        echo "How much do you want to sell?"
        get_amount $selldecimals
    done
    sellamount=$amount
    goodinp=false
    while [ $goodinp == false ]
    do
        echo "What is the minimum bid you will accept?"
        get_amount $biddecimals
    done
    minbid=$amount
    goodinp=false
    while [ $goodinp == false ]
    do
        echo "Do you want to add an optional free-form text description?"
        echo "(y)es or (n)o"
        read wantdesc
        lowcase=$(echo $wantdesc | awk '{print tolower($0)}')
        if [[ "$lowcase" == "yes" ]] || [[ "$lowcase" == "y" ]]
        then
            echo "Please enter your description without quotes"
            read desc
            descinp=",\"description\":\"$desc\""
            goodinp=true
        elif [[ "$lowcase" == "no" ]] || [[ "$lowcase" == "n" ]]
        then
            descinp=""
            goodinp=true
        fi
    done
    goodinp=false
    while [ $goodinp == false ]
    do
        echo "What label would you like to give your auction?"
        read auctionlabel
#
# change --gas amount below if getting out of gas error when creating a new auction
#
        resp=$(secretcli tx compute instantiate $contractcode "{\"sell_contract\":{\"code_hash\":\
\"$sellhash\",\"address\":\"$selladdr\"},\"bid_contract\":{\"code_hash\":\"$bidhash\",\"address\":\
\"$bidaddr\"},\"sell_amount\":\"$sellamount\",\"minimum_bid\":\"$minbid\"$descinp}" --from $addr \
            --label "$auctionlabel" --gas 300000 --broadcast-mode block --trust-node=true \
            -o json -y 2>&1)
        if echo $resp | grep "label already exists"
        then
            true
        else
            if echo $resp | grep "out of gas"
            then
                exit
            elif echo $resp | grep ERROR
            then
                exit
            elif echo $resp | grep "failed to execute message"
            then
                sendtx=$(jq -r '.txhash' <<<"$resp")
                decdsend=$(secretcli q compute tx $sendtx --trust-node=true -o json)
                jq '.output_error' <<<"$decdsend"
                exit
            else
                goodinp=true
                auctionlist=$(secretcli q compute list-contract-by-code $contractcode \
                              --trust-node=true -o json)
                auctionaddr=$(jq -r --arg AUC "$auctionlabel" '.[] | select(.label==$AUC).address'\
                    <<<"$auctionlist" )
                display_auction_info
            fi
        fi
    done
fi

exit