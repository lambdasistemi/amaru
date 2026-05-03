// Copyright 2025 PRAGMA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::PathBuf;

use amaru::{DEFAULT_NETWORK, bootstrap::import_nonces, default_chain_dir, default_initial_nonces};
use amaru_kernel::{EraHistory, NetworkName};
use clap::Parser;
use tracing::info;

#[derive(Debug, Parser)]
pub struct Args {
    /// Path of the consensus on-disk storage.
    #[arg(
        long,
        value_name = amaru::value_names::DIRECTORY,
        env = amaru::env_vars::CHAIN_DIR
    )]
    chain_dir: Option<PathBuf>,

    /// Network the nonces are imported for.
    #[arg(
        long,
        value_name = amaru::value_names::NETWORK,
        env = amaru::env_vars::NETWORK,
        default_value_t = DEFAULT_NETWORK,
    )]
    network: NetworkName,

    /// JSON-formatted file with manually specified nonces details.
    ///
    /// When not present, initial nonces are derived from the specified network, according to
    /// pre-configured values.
    #[arg(
        long,
        value_name = amaru::value_names::FILEPATH,
        env = amaru::env_vars::NONCES_FILE,
    )]
    nonces_file: Option<PathBuf>,

    /// Path to a JSON era history file overriding the network default.
    ///
    /// Required for generated testnets whose epoch length or era bounds differ from
    /// Amaru's built-in network profile. Without it, imported `Nonces` are tagged
    /// with epochs computed under the wrong epoch length, which makes the runtime
    /// `evolve_nonce` cascade fire spurious epoch-boundary branches and produce a
    /// wrong active nonce for VRF verification of the first chain-synced header.
    #[arg(
        long,
        value_name = amaru::value_names::FILEPATH,
        env = amaru::env_vars::ERA_HISTORY_FILE,
    )]
    era_history_file: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let network = args.network;

    let chain_dir = args.chain_dir.unwrap_or_else(|| default_chain_dir(network).into());

    let era_history: EraHistory = match args.era_history_file.as_deref() {
        Some(path) => serde_json::from_slice(&std::fs::read(path)?)?,
        None => <&EraHistory>::from(network).clone(),
    };

    info!(
        _command = "import-nonces",
        chain_dir = %chain_dir.to_string_lossy(),
        network = %network,
        era_history_file = %args.era_history_file.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "network default".to_string()),
        "running",
    );

    let initial_nonces = if let Some(manually_specified_file) = args.nonces_file {
        serde_json::from_slice(std::fs::read(manually_specified_file)?.as_slice())?
    } else {
        default_initial_nonces(network)?
    };

    import_nonces(&era_history, &chain_dir, initial_nonces).await
}
