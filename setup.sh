echo "Setting up Scrypto Environment and Package"

echo "\nResetting radix engine simulator..." 
resim reset

echo "\nCreating new account..."
temp_account=`resim new-account`
echo "$temp_account"
export account=`echo "$temp_account" | grep Account | grep -o "account_.*"`
export privatekey=`echo "$temp_account" | grep Private | sed "s/Private key: //"`
export account_badge=`echo "$temp_account" | grep Owner | grep -o "resource_.*"`
export xrd=`resim show $account | grep XRD | grep -o "resource_.\S*" | sed -e "s/://"`

echo "\nPublishing package..."
export package=`resim publish . | sed "s/Success! New Package: //"`

echo "\nSetup Complete"
echo "--------------------------"
echo "Environment variables set:"
echo "account = $account"
echo "privatekey = $privatekey"
echo "account_badge = $account_badge"
echo "xrd = $xrd"
echo "package = $package"

echo "\nConfiguring Mailbox"
export mailbox=`resim run manifest/create_mailbox.rtm | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}'`
export merkle_tree_hook=`resim run manifest/create_merkle_tree_hook.rtm | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}'`
export ism=`resim run manifest/create_ism.rtm | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}'`
export igp=`resim run manifest/create_igp.rtm | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}'`

resim call-method $mailbox set_required_hook $merkle_tree_hook > /dev/null
resim call-method $mailbox set_default_hook $igp > /dev/null
resim call-method $mailbox set_default_ism $ism > /dev/null
echo "mailbox = $mailbox"
echo "merkle_tree_hook = $merkle_tree_hook"
echo "igp = $igp"
echo "ism = $ism"

echo "\nSetup Warp Routes"
export noop_ism=`resim run manifest/warp/create_noop_ism.rtm | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}'`
echo "noop_ism = $noop_ism\n"

output_collateral=$(resim run manifest/warp/collateral/create_warp_collateral.rtm)
export hyp_collateral=$(echo "$output_collateral" | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}')
export hyp_collateral_owner_badge=$(echo "$output_collateral" | grep -A 2 "New Entities:" | grep "Resource:" | awk '{print $3}')
resim run manifest/warp/collateral/enroll_remote_router.rtm > /dev/null
 resim run manifest/warp/collateral/set_noop_ism.rtm

echo "hyp_collateral = $hyp_collateral"
echo "hyp_collateral_owner = $hyp_collateral_owner_badge"

output_synthetic=$(resim run manifest/warp/synthetic/create_warp_synthetic.rtm)
export hyp_synthetic=$(echo "$output_synthetic" | grep -A 1 "New Entities:" | grep "Component:" | awk '{print $3}')
export hyp_synthetic_owner_badge=$(echo "$output_synthetic" | grep -A 2 "New Entities:" | grep "Resource:" | awk '{print $3}')
export hyp_synthetic_token=$(echo "$output_synthetic" | grep -A 3 "New Entities:" | grep "Resource:" | sed -n '2p' | awk '{print $3}')
resim run manifest/warp/synthetic/enroll_remote_router.rtm > /dev/null
resim run manifest/warp/synthetic/set_noop_ism.rtm

echo ""
echo "hyp_synthetic = $hyp_synthetic"
echo "hyp_synthetic_owner_badge = $hyp_synthetic_owner_badge"
echo "hyp_synthetic_token = $hyp_synthetic_token"
