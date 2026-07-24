// Copyright 2026 PRAGMA
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

use amaru::{
    bootstrap::{import_snapshot_with_nonces, import_snapshots},
    default_ledger_dir,
};
use amaru_kernel::{HeaderHash, NetworkName};
use clap::Parser;
use tracing::info;

fn parse_header_hash(s: &str) -> Result<HeaderHash, String> {
    s.parse::<HeaderHash>().map_err(|e| e.to_string())
}

#[derive(Debug, Parser)]
pub struct Args {
    /// Path(s) to the snapshot(s) to import (CBOR file or cardano-node snapshot directory).
    #[arg(value_name = amaru::value_names::FILEPATH, required = true)]
    snapshot_paths: Vec<PathBuf>,

    /// The path to the ledger database.
    #[arg(
        long,
        value_name = amaru::value_names::DIRECTORY,
        env = amaru::env_vars::LEDGER_DIR,
    )]
    ledger_dir: Option<PathBuf>,

    /// Network of the underlying ledger database.
    #[arg(
        long,
        value_name = amaru::value_names::NETWORK,
        env = amaru::env_vars::NETWORK,
    )]
    network: NetworkName,

    /// The previous epoch's tail header hash (hex-encoded 32 bytes).
    ///
    /// When provided, nonces are extracted from the last snapshot and written
    /// to a nonces.<slot>.<hash>.json sidecar file next to it.
    #[arg(long, value_parser = parse_header_hash)]
    nonce_tail: Option<HeaderHash>,
}

#[expect(clippy::print_stdout)]
pub async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_dir = args.ledger_dir.unwrap_or_else(|| default_ledger_dir(args.network).into());

    let nonce_tail_str = args.nonce_tail.map(|t| t.to_string()).unwrap_or_default();

    info!(
        _command = "dev ledger states import",
        count = args.snapshot_paths.len(),
        ledger_dir = %ledger_dir.to_string_lossy(),
        network = %args.network,
        nonce_tail = %nonce_tail_str,
        "running",
    );

    let global_parameters = args
        .network
        .as_global_parameters()
        .ok_or_else(|| format!("no global parameters available for network {}", args.network))?;

    if let Some(nonce_tail) = args.nonce_tail {
        let (last, leading) = args
            .snapshot_paths
            .split_last()
            .ok_or("at least one snapshot path is required with --nonce-tail")?;

        if !leading.is_empty() {
            import_snapshots(args.network, global_parameters, leading, &ledger_dir).await?;
        }

        let initial_nonces =
            import_snapshot_with_nonces(args.network, global_parameters, last, &ledger_dir, nonce_tail).await?;

        let slot = initial_nonces.at.slot_or_default();
        let hash = initial_nonces.at.hash();
        let filename = format!("nonces.{}.{}.json", slot, hex::encode(hash));
        let nonces_path = last.parent().unwrap_or(std::path::Path::new(".")).join(filename);
        let json = serde_json::to_string_pretty(&initial_nonces)?;
        std::fs::write(&nonces_path, json)?;
        println!("Nonces written to {}", nonces_path.display());
    } else {
        import_snapshots(args.network, global_parameters, &args.snapshot_paths, &ledger_dir).await?;
    }

    println!("Imported {} snapshot(s) successfully", args.snapshot_paths.len());

    Ok(())
}
