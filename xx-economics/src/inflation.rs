use super::{pallet::LiquidityRewards, Config, Pallet};
use crate::BalanceOf;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::traits::{Currency, Get};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_staking::EraPayout;
use pallet_staking_reward_fn::compute_inflation;
use sp_runtime::{
	traits::{Saturating, Zero},
	Perbill, RuntimeDebug,
};
use xx_public::PublicAccountsHandler;

/// Inflation fixed parameters
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	scale_info::TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct InflationFixedParams {
	/// Minimum inflation
	#[codec(compact)]
	pub min_inflation: Perbill,
	/// Ideal stake
	#[codec(compact)]
	pub ideal_stake: Perbill,
	/// Inflation curve falloff
	#[codec(compact)]
	pub falloff: Perbill,
}

/// Default inflation fixed params of:
/// min inflation = 2.5%
/// ideal stake = 50%
/// falloff = 50%
impl Default for InflationFixedParams {
	fn default() -> Self {
		InflationFixedParams {
			min_inflation: Perbill::from_rational(1u32, 40u32),
			ideal_stake: Perbill::from_rational(1u32, 2u32),
			falloff: Perbill::from_rational(1u32, 2u32),
		}
	}
}

/// Ideal interest point
#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	scale_info::TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct IdealInterestPoint<B> {
	/// Block number
	pub block: B,
	/// Ideal inflation
	#[codec(compact)]
	pub interest: Perbill,
}

/// Default ideal interest point of:
/// block = 0
/// interest = 16.66666%
impl<B: Zero> Default for IdealInterestPoint<B> {
	fn default() -> Self {
		IdealInterestPoint { block: B::zero(), interest: Perbill::from_rational(1u32, 6u32) }
	}
}

/// Implement Inflation sub module functions
impl<T: Config> Pallet<T> {
	/// Get the ideal interest according to block number
	pub fn get_ideal_interest(block: BlockNumberFor<T>) -> Perbill {
		let points = Self::interest_points();
		match points.iter().position(|p| p.block >= block) {
			// If position found, get points from index-1 and index
			Some(index) => {
				// If index is 0, take the first point
				if index == 0 {
					points[0].interest
				} else {
					Self::compute_ideal_interest(
						block,
						points[index - 1].clone(),
						points[index].clone(),
					)
				}
			}
			// If none found, get last point and return interest
			None => points.last().unwrap().interest,
		}
	}

	/// Compute ideal interest from two points
	/// There are only two options: linear decreasing or constant
	fn compute_ideal_interest(
		block: BlockNumberFor<T>,
		start: IdealInterestPoint<BlockNumberFor<T>>,
		end: IdealInterestPoint<BlockNumberFor<T>>,
	) -> Perbill {
		// Compute interest difference (according to curve direction)
		let decreasing = start.interest > end.interest;
		let diff = if decreasing {
			start.interest.saturating_sub(end.interest)
		} else {
			end.interest.saturating_sub(start.interest)
		};
		// If difference is zero, must be constant part, take interest from start (or end)
		if diff.is_zero() {
			return start.interest
		}
		// Compute block ratio
		let block_diff = end.block - start.block;
		let half_era_blocks = Perbill::from_rational(1u32, 2u32) * T::EraDuration::get();
		// If the block is the end of the first era or higher, interpolate from the era midpoint
		// Otherwise, use the block directly
		let calc_block =
			if block >= T::EraDuration::get() { block - half_era_blocks } else { block };
		let ratio = Perbill::from_rational(calc_block, block_diff);
		// Compute interest according to curve direction
		// decreasing: start - diff*ratio
		if decreasing {
			start.interest.saturating_sub(ratio * diff)
		}
		// increasing: start + diff*ratio
		else {
			start.interest.saturating_add(ratio * diff)
		}
	}

	/// Update liquidity rewards balance
	fn update_liquidity_rewards(portion: Perbill, interest: Perbill) {
		// Calculate ideal rewards based on interest and ideal stake
		let payout = portion * (interest * Self::ideal_stake_rewards());
		// Update balance
		LiquidityRewards::<T>::mutate(|balance| *balance = balance.saturating_sub(payout));
	}

	/// Compute total stakeable
	///
	/// Total stakeable = Total Issuance - (Rewards Pool + Liquidity Rewards + Public Accounts)
	///
	/// NOTE: Custody balance is no longer subtracted as custody staking has been discontinued.
	pub fn compute_total_stakeable(issuance: BalanceOf<T>) -> BalanceOf<T> {
		let unstakeable =
            // Balance of Rewards Pool
            Self::rewards_balance()
            // add liquidity rewards balance
            + Self::liquidity_rewards()
            // add public funds accounts funds (testnet + sale)
            + T::PublicAccountsHandler::accounts().iter().fold(Zero::zero(), |acc, x| {
                acc + T::Currency::free_balance(&x)
            });
		issuance.saturating_sub(unstakeable)
	}
}

/// Implement EraPayout trait
impl<T: Config> EraPayout<BalanceOf<T>> for Pallet<T> {
	fn era_payout(
		total_staked: BalanceOf<T>,
		total_issuance: BalanceOf<T>,
		era_duration_millis: u64,
	) -> (BalanceOf<T>, BalanceOf<T>) {
		// Get inflation fixed params
		let params = Self::inflation_params();

		// Get the block number from the FRAME System module.
		let block = <frame_system::Pallet<T>>::block_number();

		// Get ideal interest for given block
		let ideal_interest = Self::get_ideal_interest(block);

		// Compute max inflation
		let max = ideal_interest * params.ideal_stake;

		// Compute total stakeable amount
		let total_stakeable = Self::compute_total_stakeable(total_issuance);

		// Ensure stake is at most total_stakeable
		let stake = total_staked.min(total_stakeable);

		// Compute stake ratio
		let stake_ratio = Perbill::from_rational(stake, total_stakeable);

		// Compute inflation
		let inflation_ratio = compute_inflation(stake_ratio, params.ideal_stake, params.falloff);
		let inflation = params
			.min_inflation
			.saturating_add(inflation_ratio * (max.saturating_sub(params.min_inflation)));

		// Milliseconds per year for the Julian year (365.25 days).
		const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
		let portion = Perbill::from_rational(era_duration_millis, MILLISECONDS_PER_YEAR);

		// Update liquidity rewards balance
		Self::update_liquidity_rewards(portion, ideal_interest);

		// Compute validator payout
		let validator_payout = portion * (inflation * total_stakeable);
		let max_payout = portion * (max * total_stakeable);
		let rest = max_payout.saturating_sub(validator_payout);

		(validator_payout, rest)
	}
}
