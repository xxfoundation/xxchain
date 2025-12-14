//! Pallet version migrations for SDK upgrade
//!
//! This module provides migrations to fix storage version mismatches between
//! the on-chain state and the new SDK pallet versions.
//!
//! ## Staking Migration Chain
//!
//! The SDK staking migrations have `in_code == X` checks that break when jumping
//! multiple versions. We provide custom wrappers that check `on_chain == X` instead.
//!
//! Migration order:
//! 1. CmixIdMigration sets version to 12
//! 2. StakingV12ToV13: 12 → 13
//! 3. StakingV13ToV14: 13 → 14
//! 4. SDK MigrateV14ToV15: 14 → 15 (VersionedMigration)
//! 5. SDK MigrateV15ToV16: 15 → 16 (VersionedMigration)

extern crate alloc;
use alloc::vec::Vec;

use frame_support::{
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};

// ============================================================================
// Staking Migrations (v12 → v16)
// ============================================================================

/// Migration from staking v12 to v13
///
/// The SDK's MigrateToV13 checks `in_code == 13` which fails when in_code is 16.
/// This wrapper checks `on_chain == 12` instead and performs the same logic:
/// - Kills the legacy StorageVersion enum (if it exists)
/// - Sets pallet version to 13
pub struct StakingV12ToV13<T>(core::marker::PhantomData<T>);

impl<T: pallet_staking::Config> OnRuntimeUpgrade for StakingV12ToV13<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();

		if on_chain == 12 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Migrating from v12 to v13"
			);

			// Kill the legacy StorageVersion enum (same as SDK v13 migration)
			frame_support::storage::unhashed::kill(&frame_support::storage::storage_prefix(
				b"Staking",
				b"StorageVersion",
			));

			StorageVersion::new(13).put::<pallet_staking::Pallet<T>>();

			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Successfully migrated to v13"
			);

			T::DbWeight::get().reads_writes(1, 2)
		} else {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Skipping v12→v13, on_chain version is {:?}",
				on_chain
			);
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();
		log::info!(
			target: "runtime::migrations::pallet_versions",
			"Staking v12→v13 pre_upgrade: on_chain = {:?}",
			on_chain
		);
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();
		log::info!(
			target: "runtime::migrations::pallet_versions",
			"Staking v12→v13 post_upgrade: on_chain = {:?}",
			on_chain
		);
		Ok(())
	}
}

/// Migration from staking v13 to v14
///
/// The SDK's MigrateToV14 checks `in_code == 14 && on_chain == 13` which fails
/// when in_code is 16. This wrapper only checks `on_chain == 13`.
/// v14 is just a version bump with no data changes.
pub struct StakingV13ToV14<T>(core::marker::PhantomData<T>);

impl<T: pallet_staking::Config> OnRuntimeUpgrade for StakingV13ToV14<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();

		if on_chain == 13 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Migrating from v13 to v14"
			);

			StorageVersion::new(14).put::<pallet_staking::Pallet<T>>();

			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Successfully migrated to v14"
			);

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Staking: Skipping v13→v14, on_chain version is {:?}",
				on_chain
			);
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();
		log::info!(
			target: "runtime::migrations::pallet_versions",
			"Staking v13→v14 pre_upgrade: on_chain = {:?}",
			on_chain
		);
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_staking::Pallet::<T>::on_chain_storage_version();
		log::info!(
			target: "runtime::migrations::pallet_versions",
			"Staking v13→v14 post_upgrade: on_chain = {:?}",
			on_chain
		);
		Ok(())
	}
}

// ============================================================================
// Other Pallet Migrations
// ============================================================================

/// Migration to bump Offences pallet from v0 to v1
pub struct OffencesV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: pallet_offences::Config> OnRuntimeUpgrade for OffencesV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_offences::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Offences: Upgrading from v0 to v1"
			);

			// Clear ReportsByKindIndex (the v1 migration clears this)
			frame_support::storage::unhashed::kill(&frame_support::storage::storage_prefix(
				b"Offences",
				b"ReportsByKindIndex",
			));

			StorageVersion::new(1).put::<pallet_offences::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 2)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_offences::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "Offences version should be at least 1");
		Ok(())
	}
}

/// Migration to bump Grandpa pallet from v4 to v5
pub struct GrandpaV4ToV5<T>(core::marker::PhantomData<T>);

impl<T: pallet_grandpa::Config> OnRuntimeUpgrade for GrandpaV4ToV5<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_grandpa::Pallet::<T>::on_chain_storage_version();

		if on_chain == 4 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Grandpa: Upgrading from v4 to v5"
			);

			StorageVersion::new(5).put::<pallet_grandpa::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_grandpa::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 5, "Grandpa version should be at least 5");
		Ok(())
	}
}

/// Migration to bump ImOnline pallet from v0 to v1
pub struct ImOnlineV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: pallet_im_online::Config> OnRuntimeUpgrade for ImOnlineV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_im_online::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"ImOnline: Upgrading from v0 to v1"
			);

			StorageVersion::new(1).put::<pallet_im_online::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_im_online::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "ImOnline version should be at least 1");
		Ok(())
	}
}

/// Migration to bump Session pallet from v0 to v1
pub struct SessionV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: pallet_session::Config> OnRuntimeUpgrade for SessionV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_session::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Session: Upgrading from v0 to v1"
			);

			StorageVersion::new(1).put::<pallet_session::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_session::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "Session version should be at least 1");
		Ok(())
	}
}

/// Migration to bump Identity pallet from v0 to v2
pub struct IdentityV0ToV2<T>(core::marker::PhantomData<T>);

impl<T: pallet_identity::Config> OnRuntimeUpgrade for IdentityV0ToV2<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_identity::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Identity: Upgrading from v0 to v2"
			);

			StorageVersion::new(2).put::<pallet_identity::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_identity::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 2, "Identity version should be at least 2");
		Ok(())
	}
}

/// Migration to bump Bounties pallet from v0 to v4
pub struct BountiesV0ToV4<T>(core::marker::PhantomData<T>);

impl<T: pallet_bounties::Config> OnRuntimeUpgrade for BountiesV0ToV4<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_bounties::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Bounties: Upgrading from v0 to v4"
			);

			StorageVersion::new(4).put::<pallet_bounties::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_bounties::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 4, "Bounties version should be at least 4");
		Ok(())
	}
}

/// Migration to bump Nfts pallet from v0 to v1
pub struct NftsV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: pallet_nfts::Config> OnRuntimeUpgrade for NftsV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_nfts::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Nfts: Upgrading from v0 to v1"
			);

			StorageVersion::new(1).put::<pallet_nfts::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_nfts::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "Nfts version should be at least 1");
		Ok(())
	}
}

/// Migration to bump XXPublic pallet from v0 to v1
pub struct XXPublicV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: xx_public::Config> OnRuntimeUpgrade for XXPublicV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = xx_public::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"XXPublic: Upgrading from v0 to v1"
			);

			StorageVersion::new(1).put::<xx_public::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = xx_public::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "XXPublic version should be at least 1");
		Ok(())
	}
}

/// Migration to bump Historical (pallet_session::historical) pallet from v0 to v1
pub struct HistoricalV0ToV1<T>(core::marker::PhantomData<T>);

impl<T: pallet_session::historical::Config> OnRuntimeUpgrade for HistoricalV0ToV1<T> {
	fn on_runtime_upgrade() -> Weight {
		let on_chain = pallet_session::historical::Pallet::<T>::on_chain_storage_version();

		if on_chain == 0 {
			log::info!(
				target: "runtime::migrations::pallet_versions",
				"Historical: Upgrading from v0 to v1"
			);

			StorageVersion::new(1).put::<pallet_session::historical::Pallet<T>>();

			T::DbWeight::get().reads_writes(1, 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let on_chain = pallet_session::historical::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(on_chain >= 1, "Historical version should be at least 1");
		Ok(())
	}
}
