//! Make the set of bag thresholds to be used with pallet-bags-list.

use clap::Parser;
use generate_bags::generate_thresholds;
use std::path::PathBuf;
use xxnetwork_runtime::Runtime as XXNetworkRuntime;

#[derive(Debug, Parser)]
// #[clap(author, version, about)]
struct Opt {
	/// How many bags to generate.
	#[clap(long, default_value = "200")]
	n_bags: usize,

	/// Where to write the output.
	output: PathBuf,

	/// The total issuance of the native currency.
	#[clap(short, long)]
	total_issuance: u128,

	/// The minimum account balance (i.e. existential deposit) for the native curenc.y
	#[clap(short, long)]
	minimum_balance: u128,
}

fn main() -> Result<(), std::io::Error> {
	let Opt { n_bags, output, total_issuance, minimum_balance } = Opt::parse();
	generate_thresholds::<XXNetworkRuntime>(n_bags, &output, total_issuance, minimum_balance)
}
