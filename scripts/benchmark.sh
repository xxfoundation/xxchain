#!/bin/bash

# Benchmark script updated for Polkadot SDK polkadot-stable2509-2
# Uses --chain dev to get properly funded genesis state for benchmarks
#
# NOTE: The following warnings during benchmarking are EXPECTED and intentional:
#   - "WARNING: benchmark error overridden - remove_member_without_replacement"
#     This is intentional SDK behavior. The pallet_elections_phragmen benchmark
#     returns BenchmarkError::Override with max_block weight for this edge case
#     operation (removing a member when there are no runners-up to replace them).
#     The max weight acts as a deterrent since this shouldn't happen normally.

set -e

OUTPUT_DIR="./runtime/xxnetwork/src/weights"

echo "Running xx network Runtime benchmarks"
echo "Chain: dev (with funded accounts for benchmarks)"
echo "Output: $OUTPUT_DIR"

# Build the binary if needed
if [ ! -f ./target/release/xxnetwork-chain ]; then
    echo "Building release binary with runtime-benchmarks feature..."
    cargo build --release --features=runtime-benchmarks
fi

# List of pallets to benchmark (excluding benchmarking infrastructure pallets)
PALLETS=(
    "frame_system"
    "pallet_balances"
    "pallet_timestamp"
    "pallet_scheduler"
    "pallet_preimage"
    "pallet_democracy"
    "pallet_collective"
    "pallet_elections_phragmen"
    "pallet_membership"
    "pallet_treasury"
    "pallet_bounties"
    "pallet_child_bounties"
    "pallet_tips"
    "pallet_staking"
    "pallet_session"
    "pallet_bags_list"
    "pallet_election_provider_multi_phase"
    "pallet_im_online"
    "pallet_identity"
    "pallet_vesting"
    "pallet_proxy"
    "pallet_multisig"
    "pallet_utility"
    "pallet_recovery"
    "pallet_assets"
    "pallet_uniques"
    "pallet_nfts"
    "claims"
    "swap"
    "xx_cmix"
    "xx_economics"
    "xx_team_custody"
    "xx_betanet_rewards"
    "xx_public"
    "frame_election_provider_support"
)

mkdir -p "$OUTPUT_DIR"

for pallet in "${PALLETS[@]}"; do
    echo "============================================"
    echo "Benchmarking: $pallet"
    echo "============================================"

    # Convert pallet name for output filename (:: -> _)
    output_file="${OUTPUT_DIR}/${pallet/::/_}.rs"

    # Special handling for pallet_bags_list: skip on_idle benchmark
    # The on_idle benchmark requires MaxAutoRebagPerBlock > 0, but we have it set to 0.
    # We keep a placeholder weight for on_idle since it does nothing in production.
    if [ "$pallet" = "pallet_bags_list" ]; then
        ./target/release/xxnetwork-chain benchmark pallet \
            --chain dev \
            --pallet "$pallet" \
            --extrinsic "rebag_non_terminal,rebag_terminal,put_in_front_of" \
            --steps 50 \
            --repeat 20 \
            --heap-pages 4096 \
            --output "$output_file" \
            || echo "Warning: Failed to benchmark $pallet"

        # Insert on_idle placeholder before the final closing brace
        # The benchmark output ends with "}\n" so we insert our function before the last line
        sed -i.bak '$d' "$output_file"  # Remove last line (closing brace)
        echo "	// Placeholder: on_idle does nothing when MaxAutoRebagPerBlock = 0" >> "$output_file"
        echo "	fn on_idle() -> Weight { Weight::from_parts(5_000_000, 0) }" >> "$output_file"
        echo "}" >> "$output_file"
        rm -f "${output_file}.bak"
        continue
    fi

    ./target/release/xxnetwork-chain benchmark pallet \
        --chain dev \
        --pallet "$pallet" \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --heap-pages 4096 \
        --output "$output_file" \
        || echo "Warning: Failed to benchmark $pallet"
done

echo "============================================"
echo "Benchmarking complete!"
echo "Weight files written to: $OUTPUT_DIR"
echo "============================================"
