// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
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

//! Utilities to build a `TestClient` for `node-runtime`.

use sp_runtime::BuildStorage;
/// Re-export test-client utilities.
pub use substrate_test_client::*;

/// The WasmExecutor type used for test clients.
pub type WasmExecutorType = sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>;

/// Default backend type.
pub type Backend = sc_client_db::Backend<node_primitives::Block>;

/// Call executor type wrapping WasmExecutor in LocalCallExecutor.
pub type ExecutorDispatch = sc_service::client::LocalCallExecutor<
	node_primitives::Block,
	Backend,
	WasmExecutorType,
>;

/// Test client type using LocalCallExecutor with WasmExecutor.
pub type Client = substrate_test_client::client::Client<
	Backend,
	ExecutorDispatch,
	node_primitives::Block,
	xxnetwork_runtime::RuntimeApi,
>;

/// Genesis configuration parameters for `TestClient`.
#[derive(Default)]
pub struct GenesisParameters;

impl substrate_test_client::GenesisInit for GenesisParameters {
	fn genesis_storage(&self) -> Storage {
		crate::genesis::config(None).build_storage().unwrap()
	}
}

/// Type alias for TestClientBuilder with our parameters.
/// Note: TestClientBuilder expects LocalCallExecutor<Block, Backend, WasmExecutor<H>>
/// to use build_with_native_executor method.
pub type TestClientBuilder = substrate_test_client::TestClientBuilder<
	node_primitives::Block,
	ExecutorDispatch,
	Backend,
	GenesisParameters,
>;

/// Build a test client with default settings.
pub fn new_client() -> Client {
	let executor: WasmExecutorType = sc_executor::WasmExecutor::builder()
		.build();
	TestClientBuilder::default()
		.build_with_native_executor(executor)
		.0
}
