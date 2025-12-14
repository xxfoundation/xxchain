// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
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

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

use futures::prelude::*;
use node_primitives::Block;
use sc_client_api::BlockBackend;
use sc_consensus_babe::{self, SlotProportion};
use sc_consensus_grandpa as grandpa;
use sc_network::{event::Event, NetworkEventStream};
use sc_service::{
	config::Configuration, error::Error as ServiceError, TaskManager,
};
use sp_consensus_babe::inherents::BabeCreateInherentDataProviders;
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use std::sync::Arc;

pub use node_executor::XXNetworkExecutorDispatch;
pub use xxnetwork_runtime::RuntimeApi as XXNetworkRuntimeApi;

/// Host functions for runtime
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
);

/// Runtime executor type
pub type RuntimeExecutor = sc_executor::WasmExecutor<HostFunctions>;

// Common types
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type FullClient = sc_service::TFullClient<Block, XXNetworkRuntimeApi, RuntimeExecutor>;
type FullGrandpaBlockImport = grandpa::GrandpaBlockImport<
	FullBackend,
	Block,
	FullClient,
	FullSelectChain
>;
type TransactionPool = sc_transaction_pool::TransactionPoolHandle<Block, FullClient>;

/// The minimum period of blocks on which justifications will be imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// Creates a partial service from the configuration.
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		TransactionPool,
		(
			impl Fn(
				sc_rpc::SubscriptionTaskExecutor,
			) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>,
			(
				sc_consensus_babe::BabeBlockImport<
					Block,
					FullClient,
					FullGrandpaBlockImport,
					BabeCreateInherentDataProviders<Block>,
					FullSelectChain,
				>,
				grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
				sc_consensus_babe::BabeLink<Block>,
				sc_consensus_babe::BabeWorkerHandle<Block>,
			),
			grandpa::SharedVoterState,
			Option<Telemetry>,
		)
	>,
	ServiceError,
> {
	let telemetry = config.telemetry_endpoints.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, XXNetworkRuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
			task_manager.spawn_handle().spawn("telemetry", None, worker.run());
			telemetry
		});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::from(sc_transaction_pool::Builder::new(
		task_manager.spawn_essential_handle(),
		client.clone(),
		config.role.is_authority().into(),
	)
	.with_options(config.transaction_pool.clone())
	.with_prometheus(config.prometheus_registry())
	.build());

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let babe_config = sc_consensus_babe::configuration(&*client)?;
	let slot_duration = babe_config.slot_duration();
	let (block_import, babe_link) = sc_consensus_babe::block_import(
		babe_config,
		grandpa_block_import,
		client.clone(),
		Arc::new(move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);
			Ok((slot, timestamp))
		}) as BabeCreateInherentDataProviders<Block>,
		select_chain.clone(),
		OffchainTransactionPoolFactory::new(transaction_pool.clone()),
	)?;

	let (import_queue, babe_worker_handle) = sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
		link: babe_link.clone(),
		block_import: block_import.clone(),
		justification_import: Some(Box::new(justification_import)),
		client: client.clone(),
		slot_duration,
		spawner: &task_manager.spawn_essential_handle(),
		registry: config.prometheus_registry(),
		telemetry: telemetry.as_ref().map(|x| x.handle()),
	})?;

	let import_setup = (block_import, grandpa_link, babe_link, babe_worker_handle.clone());

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link, _, _) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();

		let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_extensions_builder = move |subscription_executor: sc_rpc::SubscriptionTaskExecutor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				babe: node_rpc::BabeDeps {
					babe_worker_handle: babe_worker_handle.clone(),
					keystore: keystore.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
			};

			node_rpc::create_full(deps).map_err(Into::into)
		};

		(rpc_extensions_builder, shared_voter_state2)
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (rpc_extensions_builder, import_setup, rpc_setup, telemetry),
	})
}

/// Creates a full service from the configuration.
pub fn new_full_base(
	config: Configuration,
	disable_hardware_benchmarks: bool,
) -> Result<TaskManager, ServiceError> {
	let hwbench = if !disable_hardware_benchmarks {
		config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(&database_path);
			sc_sysinfo::gather_hwbench(
				Some(database_path),
				&frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE,
			)
		})
	} else {
		None
	};

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (rpc_builder, import_setup, rpc_setup, mut telemetry),
	} = new_partial(&config)?;

	let shared_voter_state = rpc_setup;
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let grandpa_protocol_name = grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	let mut net_config = sc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as sp_runtime::traits::Block>::Hash,
		sc_network::Litep2pNetworkBackend,
	>::new(&config.network, config.prometheus_registry().cloned());

	let peer_store_handle = net_config.peer_store_handle();
	let metrics = sc_network::NotificationMetrics::new(config.prometheus_registry());

	let (grandpa_notification_protocol, grandpa_notification_service) =
		grandpa::grandpa_peers_set_config::<Block, sc_network::Litep2pNetworkBackend>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(grandpa_notification_protocol);

	let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(sc_network_sync::WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	let role = config.role;
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
			config,
			backend: backend.clone(),
			client: client.clone(),
			keystore: keystore_container.keystore(),
			network: network.clone(),
			rpc_builder: Box::new(rpc_builder),
			transaction_pool: transaction_pool.clone(),
			task_manager: &mut task_manager,
			system_rpc_tx,
			tx_handler_controller,
			sync_service: sync_service.clone(),
			telemetry: telemetry.as_mut(),
		})?;

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	let (block_import, grandpa_link, babe_link, _) = import_setup;

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |_parent, ()| {
				let _client_clone = client_clone.clone();
				async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					Ok((slot, timestamp))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
		    sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
		    network.event_stream("authority-discovery").filter_map(|e| async move {
		        match e {
                    Event::Dht(e) => Some(e),
                    _ => None,
                    }
                });
		let (authority_discovery_worker, _service) =
		    sc_authority_discovery::new_worker_and_service_with_config(
                sc_authority_discovery::WorkerConfig {
                    publish_non_global_ips: auth_disc_publish_non_global_ips,
                    ..Default::default()
                },
                client.clone(),
                Arc::new(network.clone()),
                Box::pin(dht_event_stream),
                authority_discovery_role,
                prometheus_registry.clone(),
                task_manager.spawn_handle(),
            );

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore =
		if role.is_authority() { Some(keystore_container.keystore()) } else { None };

	let grandpa_config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_params = grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			sync: Arc::new(sync_service),
			notification_service: grandpa_notification_service,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			grandpa::run_grandpa_voter(grandpa_params)?,
		);
	}

	Ok(task_manager)
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	disable_hardware_benchmarks: bool,
) -> Result<TaskManager, ServiceError> {
	new_full_base(config, disable_hardware_benchmarks)
}
