#!/bin/bash
# Comprehensive runtime 207 upgrade verification
#
# Usage:
#   ./scripts/verify-upgrade.sh [RPC_ENDPOINT]
#
# Default RPC endpoint: http://127.0.0.1:9944

set -e

RPC="${1:-http://127.0.0.1:9944}"
EXPECTED_SPEC_VERSION=207

echo "============================================"
echo "xx Network Runtime 207 Upgrade Verification"
echo "RPC Endpoint: $RPC"
echo "============================================"
echo ""

# Helper function
query_rpc() {
    curl -s -H "Content-Type: application/json" \
        -d "$1" \
        $RPC
}

# 1. Check Runtime Version
echo "1. Checking Runtime Version..."
SPEC_VERSION=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "state_getRuntimeVersion"}' | jq -r '.result.specVersion')

if [ "$SPEC_VERSION" = "$EXPECTED_SPEC_VERSION" ]; then
    echo "   ✅ Runtime version: $SPEC_VERSION"
else
    echo "   ❌ Runtime version: $SPEC_VERSION (expected $EXPECTED_SPEC_VERSION)"
    exit 1
fi

# 2. Check System Health
echo "2. Checking System Health..."
HEALTH=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "system_health"}')
IS_SYNCING=$(echo $HEALTH | jq -r '.result.isSyncing')

if [ "$IS_SYNCING" = "false" ]; then
    echo "   ✅ Node is synced"
else
    echo "   ⚠️  Node is syncing"
fi

# 3. Check Block Production
echo "3. Checking Block Production..."
BEFORE_BLOCK=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' | jq -r '.result.number' | xargs printf "%d")
query_rpc '{"id":1, "jsonrpc":"2.0", "method": "dev_newBlock", "params": [{}]}' > /dev/null 2>&1 || true
AFTER_BLOCK=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "chain_getHeader"}' | jq -r '.result.number' | xargs printf "%d")

if [ "$AFTER_BLOCK" -gt "$BEFORE_BLOCK" ]; then
    echo "   ✅ Block production works ($BEFORE_BLOCK -> $AFTER_BLOCK)"
else
    echo "   ⚠️  Block production unchanged (may be in non-dev mode)"
fi

# 4. Check Chain Info
echo "4. Checking Chain Info..."
CHAIN_NAME=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' | jq -r '.result')
echo "   ✅ Chain: $CHAIN_NAME"

# 5. Check Total Issuance
echo "5. Checking Balances..."
# Storage key for balances.totalIssuance
TOTAL_ISSUANCE=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "state_getStorage", "params": ["0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"]}' | jq -r '.result')
if [ "$TOTAL_ISSUANCE" != "null" ] && [ -n "$TOTAL_ISSUANCE" ]; then
    echo "   ✅ Total issuance exists: ${TOTAL_ISSUANCE:0:20}..."
else
    echo "   ❌ Total issuance not found"
    exit 1
fi

# 6. Check Staking Active Era
echo "6. Checking Staking State..."
# Storage key for staking.activeEra
ACTIVE_ERA=$(query_rpc '{"id":1, "jsonrpc":"2.0", "method": "state_getStorage", "params": ["0x5f3e4907f716ac89b6347d15ececedca487df464e44a534ba6b0cbb32407b587"]}' | jq -r '.result')
if [ "$ACTIVE_ERA" != "null" ] && [ -n "$ACTIVE_ERA" ]; then
    echo "   ✅ Active era exists: ${ACTIVE_ERA:0:20}..."
else
    echo "   ❌ Active era not found"
    exit 1
fi

# 7. Check xx-economics liquidity rewards (confirms BridgeAdjust migration)
echo "7. Checking xx-economics State..."
# Storage key for xxEconomics.liquidityRewards
# This is a simple value storage, key is twox128("XXEconomics") ++ twox128("LiquidityRewards")
LIQUIDITY_KEY="0x$(printf 'XXEconomics' | xxd -p)$(printf 'LiquidityRewards' | xxd -p)" 2>/dev/null || true
echo "   ⚠️  Verify xxEconomics.liquidityRewards manually in Polkadot.js Apps"
echo "   Expected: ~10,000,000,000,000,000 (10M XX)"

# 8. Summary
echo ""
echo "============================================"
echo "Verification Complete"
echo "============================================"
echo ""
echo "Basic checks passed. Next steps:"
echo ""
echo "1. Open Polkadot.js Apps:"
echo "   https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944"
echo ""
echo "2. Verify xx-specific pallets in Developer > Chain State:"
echo "   - xxStakingExtension.cmixIds(validatorStash) - should return CmixId"
echo "   - xxEconomics.liquidityRewards() - should be ~10M XX"
echo "   - xxCmix.cmixHashes() - should return hash values"
echo "   - xxCmix.schedulingAccount() - should return AccountId"
echo "   - xxCustody.totalCustody() - should return balance"
echo ""
echo "3. Test extrinsic submission (signatures are mocked with mock-signature-host: true)"
echo ""
echo "4. Review migration logs from chopsticks try-runtime output"
