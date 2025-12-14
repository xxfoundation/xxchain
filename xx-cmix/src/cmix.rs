use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{Permill, RuntimeDebug};

/// CMIX software hashes
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct SoftwareHashes<Hash> {
	/// Server binary
	pub(crate) server: Hash,
	/// FatBin library
	pub(crate) fatbin: Hash,
	/// Libpow library
	pub(crate) libpow: Hash,
	/// Gateway binary
	pub(crate) gateway: Hash,
	/// Scheduling server binary
	pub(crate) scheduling: Hash,
	/// Wrapper script
	pub(crate) wrapper: Hash,
	/// User discovery bot binary
	pub(crate) udb: Hash,
	/// Notifications bot binary
	pub(crate) notifications: Hash,
	/// Extra
	pub(crate) extra: Option<Vec<Hash>>,
}

/// Country code type
type CountryCode = [u8; 2];

/// Geographic bin type
type GeoBin = u8;

/// Points multiplier type
type PointsMultiplier = u16;

/// Reward Points
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct RewardPoints {
	/// Points per successful real-time round
	pub(crate) success: u32,
	/// Points per failed real-time round (negative)
	pub(crate) failure: u32,
	/// Points per block produced
	pub(crate) block: u32,
}

/// Performance measurement variables
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct Performance {
	/// Period for round performance data collection
	pub(crate) period: u64,
	/// Reward points
	pub(crate) points: RewardPoints,
	/// List of countries and their geographic bins
	pub(crate) countries: Vec<(CountryCode, GeoBin)>,
	/// List of geographic bins and their points multiplier
	pub(crate) multipliers: Vec<(GeoBin, PointsMultiplier)>,
}

/// Round Timeouts
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct Timeouts {
	/// Round precomputation timeout
	pub(crate) precomputation: u64,
	/// Round realtime timeout
	pub(crate) realtime: u64,
	/// Round advertisement time
	pub(crate) advertisement: u64,
}

/// Scheduling variables
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct Scheduling {
	/// Mix team size
	pub(crate) team_size: u8,
	/// Mix batch size
	pub(crate) batch_size: u32,
	/// Minimum delay between round assignments
	pub(crate) min_delay: u64,
	/// Minimum number of nodes in the waiting pool before rounds can be scheduled
	pub(crate) pool_threshold: Permill,
}

/// User Registration variables
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct UserRegistration {
	/// Maximum number of user registrations per period
	pub(crate) max: u32,
	/// Period of user registration
	pub(crate) period: u64,
}

/// CMIX Variables
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Default,
	RuntimeDebug,
	TypeInfo,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct Variables {
	/// Performance measurement variables
	pub(crate) performance: Performance,
	/// Round timeouts
	pub(crate) timeouts: Timeouts,
	/// Scheduling
	pub(crate) scheduling: Scheduling,
	/// User registration
	pub(crate) registration: UserRegistration,
}

impl Variables {
	pub fn get_block_points(&self) -> u32 {
		self.performance.points.block
	}
}
