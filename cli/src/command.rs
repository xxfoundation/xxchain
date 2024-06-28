// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{chain_spec, service, Cli, Subcommand};
use sc_cli::{Result, SubstrateCli, RuntimeVersion, ChainSpec};
use sc_service::PartialComponents;
use crate::chain_spec::IdentifyVariant;
use frame_benchmarking_cli::*;
use std::sync::Arc;

#[cfg(feature = "try-runtime")]
use {
	runtime_common::constants::time::SLOT_DURATION,
	try_runtime_cli::block_building_info::substrate_info,
};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"xxlabs xxnetwork".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/xx-labs/xxchain/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2020
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let id = if id == "" {
			"xxnetwork"
		} else { id };
		Ok(match id {
			#[cfg(feature = "xxnetwork")]
			"xxnetwork" => Box::new(chain_spec::xxnetwork_config()?),
			#[cfg(feature = "xxnetwork")]
			"xxnetwork-dev" | "dev" => Box::new(chain_spec::xxnetwork_development_config()),
			#[cfg(feature = "canary")]
			"canary" => Box::new(chain_spec::canary_config()?),
			#[cfg(feature = "canary")]
			"canary-dev" => Box::new(chain_spec::canary_development_config()),
			path => {
				let path = std::path::PathBuf::from(path);

				let chain_spec = Box::new(chain_spec::XXNetworkChainSpec::from_json_file(path.clone())?) as Box<dyn sc_chain_spec::ChainSpec>;

				// When the file name starts with the name of one of the known chains,
				// we use the chain spec for the specific chain.
				if chain_spec.is_canary() {
					Box::new(chain_spec::CanaryChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			},
		})
	}

	fn native_runtime_version(spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		#[cfg(feature = "canary")]
		if spec.is_canary() {
			return &chain_spec::canary::VERSION
		}

		#[cfg(not(feature = "canary"))]
		let _ = spec;

		#[cfg(feature = "xxnetwork")]
		{
			return &chain_spec::xxnetwork::VERSION
		}

		#[cfg(not(feature = "xxnetwork"))]
		panic!("No runtime feature (xxnetwork, canary) is enabled")
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			log::info!("Running node");
			runner.run_node_until_exit(|config| async move {
				service::new_full(config, cli.no_hardware_benchmarks)
					.map_err(sc_cli::Error::Service)
			})
		},
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.sync_run(|config|
						cmd.run::<
							chain_spec::canary::Block,
							chain_spec::canary::RuntimeApi,
							service::CanaryExecutorDispatch>(config)
					)
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.sync_run(|config|
					cmd.run::<
						chain_spec::xxnetwork::Block,
						chain_spec::xxnetwork::RuntimeApi,
						service::XXNetworkExecutorDispatch>(config)
				)
			}
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							)
						}
						#[cfg(feature = "canary")]
						{
							let chain_spec = &config.chain_spec;
							if chain_spec.is_canary() {
								return cmd.run::<
									chain_spec::canary::Block,
									service::CanaryExecutorDispatch
								>(config)
							};
							#[cfg(not(feature = "xxnetwork"))]
							return Err("Chain spec doesn't match canary runtime!".into())
						}
						#[cfg(feature = "xxnetwork")]
						{
							cmd.run::<
								chain_spec::xxnetwork::Block,
								service::XXNetworkExecutorDispatch
							>(config)
						}
					},
					_ => {
						return Err("Benchmark subcomamnd not supported".into())
					}
				}
			})
		}
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						let PartialComponents { client, task_manager, import_queue, ..}
							= service::new_partial::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>(&config)?;
						Ok((cmd.run(client, import_queue), task_manager))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					let PartialComponents { client, task_manager, import_queue, ..}
						= service::new_partial::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>(&config)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						let PartialComponents { client, task_manager, ..}
							= service::new_partial::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>(&config)?;
						Ok((cmd.run(client, config.database), task_manager))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					let PartialComponents { client, task_manager, ..}
						= service::new_partial::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>(&config)?;
					Ok((cmd.run(client, config.database), task_manager))
				})
			}
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						let PartialComponents { client, task_manager, ..}
							= service::new_partial::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>(&config)?;
						Ok((cmd.run(client, config.chain_spec), task_manager))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					let PartialComponents { client, task_manager, ..}
						= service::new_partial::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>(&config)?;
					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			}
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						let PartialComponents { client, task_manager, import_queue, ..}
							= service::new_partial::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>(&config)?;
						Ok((cmd.run(client, import_queue), task_manager))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					let PartialComponents { client, task_manager, import_queue, ..}
						= service::new_partial::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>(&config)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						let PartialComponents { client, task_manager, backend, ..}
							= service::new_partial::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>(&config)?;
						let aux_revert = Box::new(|client: Arc<service::FullClient::<service::CanaryRuntimeApi, service::CanaryExecutorDispatch>>, backend, blocks| {
							sc_consensus_babe::revert(client.clone(), backend, blocks)?;
							grandpa::revert(client, blocks)?;
							Ok(())
						});
						Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					let PartialComponents { client, task_manager, backend, ..}
						= service::new_partial::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>(&config)?;
					let aux_revert = Box::new(|client: Arc<service::FullClient::<service::XXNetworkRuntimeApi, service::XXNetworkExecutorDispatch>>, backend, blocks| {
						sc_consensus_babe::revert(client.clone(), backend, blocks)?;
						grandpa::revert(client, blocks)?;
						Ok(())
					});
					Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
				})
			}
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
			let runner = cli.create_runner(cmd)?;
			let info_provider = substrate_info(SLOT_DURATION);
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.async_run(|config| {
						// we don't need any of the components of new_partial, just a runtime, or a task
						// manager to do `async_run`.
						let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
						let task_manager =
							sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
								.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

						Ok((
							cmd.run::<chain_spec::canary::Block, ExtendedHostFunctions<
								sp_io::SubstrateHostFunctions,
								<service::CanaryExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
							>, _>(Some(info_provider)),
							task_manager,
						))
					})
				};
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.async_run(|config| {
					// we don't need any of the components of new_partial, just a runtime, or a task
					// manager to do `async_run`.
					let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
					let task_manager =
						sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
							.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

					Ok((
						cmd.run::<chain_spec::xxnetwork::Block, ExtendedHostFunctions<
							sp_io::SubstrateHostFunctions,
							<service::XXNetworkExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
						>, _>(Some(info_provider)),
						task_manager,
					))
				})
			}
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			#[cfg(feature = "canary")]
			{
				let chain_spec = &runner.config().chain_spec;
				if chain_spec.is_canary() {
					return runner.sync_run(|config| cmd.run::<chain_spec::canary::Block>(&config))
				}
				#[cfg(not(feature = "xxnetwork"))]
				return Err("Chain spec doesn't match canary runtime!".into())
			}
			#[cfg(feature = "xxnetwork")]
			{
				return runner.sync_run(|config| cmd.run::<chain_spec::xxnetwork::Block>(&config))
			}
		},
	}
}
