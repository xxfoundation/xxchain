//! Weights for `pallet_revive`
//!
//! This file uses the SDK's built-in SubstrateWeight implementation.
//! Run benchmarks post-deployment to generate chain-specific weights:
//!
//! ```bash
//! ./target/release/xxnetwork-chain benchmark pallet \
//!     --chain=dev \
//!     --pallet=pallet_revive \
//!     --extrinsic=* \
//!     --steps=50 \
//!     --repeat=20 \
//!     --heap-pages=4096 \
//!     --output=runtime/xxnetwork/src/weights/pallet_revive.rs
//! ```

// Re-export SDK's SubstrateWeight for pallet-revive
// The runtime uses pallet_revive::weights::SubstrateWeight<Runtime> directly in Config
// This module exists for consistency with other weight modules
