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

use std::error::Error;

use tokio::sync::OnceCell;

/// An HTTP client that is only constructed once something actually needs it.
///
/// `snapshot create` reaches the network on two paths: downloading the official
/// cardano-node configuration bundle, and resolving snapshot points from Koios.
/// Both are skipped when the caller supplies `--cardano-node-config-dir` and
/// `--snapshot`, which is the only usable combination for a custom testnet
/// since Koios serves mainnet, preprod and preview only.
///
/// Constructing the client eagerly made that fully-offline path depend on a
/// usable system CA store anyway: `reqwest::Client::new()` *panics* when no CA
/// certificates can be loaded, so an offline bootstrap aborted before doing any
/// work. Deferring construction keeps the offline path free of TLS setup, and
/// building through the fallible builder turns a genuine misconfiguration into
/// an ordinary error rather than a panic.
#[derive(Default)]
pub(super) struct LazyClient(OnceCell<reqwest::Client>);

impl LazyClient {
    /// Return the shared client, building it on first use.
    pub(super) async fn get(&self) -> Result<&reqwest::Client, Box<dyn Error>> {
        self.0.get_or_try_init(|| async { reqwest::Client::builder().build() }).await.map_err(Into::into)
    }

    /// Whether the client has been built yet.
    #[cfg(test)]
    pub(super) fn is_initialized(&self) -> bool {
        self.0.initialized()
    }
}

#[cfg(test)]
mod tests {
    use super::LazyClient;

    #[test]
    fn starts_uninitialized() {
        // The offline path constructs a LazyClient and never touches it; that
        // must not build (nor panic while building) a TLS-backed client.
        assert!(!LazyClient::default().is_initialized());
    }

    #[tokio::test]
    async fn get_initializes_once() {
        let client = LazyClient::default();
        assert!(!client.is_initialized());

        let first = client.get().await.expect("client builds") as *const reqwest::Client;
        assert!(client.is_initialized());

        let second = client.get().await.expect("client builds") as *const reqwest::Client;
        assert_eq!(first, second, "the client is built once and shared");
    }
}
