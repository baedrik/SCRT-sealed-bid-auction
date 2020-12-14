#!/bin/bash

# Change this to the code ID of the auction contract for whatever chain your secretcli is using
contractcode="17"

cat << EOF
Just a reminder that you need to have secretcli and jq installed.
You can install jq with:  sudo apt-get install jq

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

# used to input check numbers
re='^[0-9]+$'

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
    auctioninfo=$(secretcli q compute query $auctionaddr '{"auction_info":{}}' --trust-node=true \
                  -o json)
    jq <<<"$auctioninfo"
    sellcontr=$(jq -r '.[].sell_token.contract_address' <<<"$auctioninfo")
    bidcontr=$(jq -r '.[].bid_token.contract_address' <<<"$auctioninfo")
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
            jq <<<"$fnlresp"
# consign tokens
        elif [[ $owncmd == 'c' ]]
        then
            goodinp=false
            while [ $goodinp == false ]
            do
                echo "How much do you want to consign (in lowest denomination of sell token)?"
                echo "Recommend consigning the full sale amount, but you can do it in multiple"
                echo "transactions if you want"
                read csnamount
                if [[ "$csnamount" =~ $re ]]
                then
                   goodinp=true
                else
                   echo -e "\nINPUT MUST BE NUMERIC"
                fi
            done
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
                jq <<<"$cleaned"
            else
                echo $decdsenderr
            fi
# display auction info
        elif [[ $owncmd == 'd' ]]
        then
            auctioninfo=$(secretcli q compute query $auctionaddr '{"auction_info":{}}'\
                          --trust-node=true -o json)
            jq <<<"$auctioninfo"
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
                echo "How much do you want to bid (in lowest denomination of bid token)?"
                read bidamount
                if [[ "$bidamount" =~ $re ]]
                then
                   goodinp=true
                else
                   echo -e "\nINPUT MUST BE NUMERIC"
                fi
            done

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
                jq <<<"$cleaned"
            else
                echo $decdsenderr
            fi
# display auction info
        elif [[ $bidcmd == 'd' ]]
        then
            jq <<<"$auctioninfo"
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
            jq <<<"$bidresp"
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
            jq <<<"$bidresp"
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
        read inp
        sellhash=$(secretcli q compute contract-hash $inp --trust-node=true -o json 2>&1)
        if echo $sellhash | grep ERROR
        then
            true
        else
            sellhash=${sellhash/0x/}
            selladdr=$inp
            goodinp=true
        fi
    done
    goodinp=false
    while [ $goodinp == false ]
    do
        echo -e "\nWhat is the contract address of the token you will accept bids in?"
        read inp
        bidhash=$(secretcli q compute contract-hash $inp --trust-node=true -o json 2>&1)
        if echo $bidhash | grep ERROR
        then
            true
        else
            bidhash=${bidhash/0x/}
            bidaddr=$inp
            goodinp=true
        fi
    done
    goodinp=false
    while [ $goodinp == false ]
    do
        echo "How much do you want to sell (in lowest denomination of sale token)?"
        read sellamount
        if [[ "$sellamount" =~ $re ]]
        then
           goodinp=true
        else
           echo -e "\nINPUT MUST BE NUMERIC"
        fi
    done
    goodinp=false
    while [ $goodinp == false ]
    do
        echo "What is the minimum bid you will accept (in lowest denomination of bid token)?"
        read minbid
        if [[ "$minbid" =~ $re ]]
        then
           goodinp=true
        else
           echo -e "\nINPUT MUST BE NUMERIC"
        fi
    done
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
                auctioninfo=$(secretcli q compute query $auctionaddr '{"auction_info":{}}' \
                              --trust-node=true -o json)
                jq <<<"$auctioninfo"
            fi
        fi
    done
fi
