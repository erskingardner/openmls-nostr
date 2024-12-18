//! OpenMLS Nostr is a library that simplifies the implmentation of NIP-104 Nostr Groups using the OpenMLS implementation of the MLS protocol.
//! It's expected that you'd use this library along with the [Rust Nostr library](https://github.com/rust-nostr/nostr).

use openmls::prelude::*;
use openmls_rust_crypto::RustCrypto;
use openmls_sled_storage::{SledStorage, SledStorageError};
use std::path::PathBuf;
use thiserror::Error;

pub mod groups;
pub mod key_packages;
pub mod nostr_group_data_extension;
pub mod welcomes;

#[cfg(test)]
pub mod test_utils;

#[derive(Debug, Error)]
pub enum NostrMlsError {
    #[error("Error updating provider for user: {0}")]
    ProviderError(String),
}

/// The main struct for the Nostr MLS implementation.
pub struct NostrMls {
    /// The ciphersuite to use
    pub ciphersuite: Ciphersuite,
    /// The required extensions
    pub extensions: Vec<ExtensionType>,
    /// An implementation of the OpenMls provider trait
    pub provider: NostrMlsProvider,
}

/// The provider struct for Nostr MLS that implements the OpenMLS Provider trait.
pub struct NostrMlsProvider {
    crypto: RustCrypto,
    key_store: SledStorage,
}

impl OpenMlsProvider for NostrMlsProvider {
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;
    type StorageProvider = SledStorage;

    fn storage(&self) -> &Self::StorageProvider {
        &self.key_store
    }

    fn crypto(&self) -> &Self::CryptoProvider {
        &self.crypto
    }

    fn rand(&self) -> &Self::RandProvider {
        &self.crypto
    }
}

impl NostrMls {
    /// Default ciphersuite for Nostr Groups.
    /// This is also the only required ciphersuite for Nostr Groups.
    const DEFAULT_CIPHERSUITE: Ciphersuite =
        Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

    /// Required extensions for Nostr Groups.
    const REQUIRED_EXTENSIONS: &[ExtensionType] = &[
        ExtensionType::RequiredCapabilities,
        ExtensionType::LastResort,
        ExtensionType::RatchetTree,
        ExtensionType::Unknown(0xF233), // Nostr Group Data Extension
    ];

    /// GREASE values for MLS.
    #[allow(dead_code)] // TODO: Remove this once we've added GREASE support.
    const GREASE: &[u16] = &[
        0x0A0A, 0x1A1A, 0x2A2A, 0x3A3A, 0x4A4A, 0x5A5A, 0x6A6A, 0x7A7A, 0x8A8A, 0x9A9A, 0xAAAA,
        0xBABA, 0xCACA, 0xDADA, 0xEAEA,
    ];

    pub fn new(storage_path: PathBuf, active_identity: Option<String>) -> Self {
        // We want MLS data to be stored on a per user basis so we create a new path
        // and hence a new database instance for each user.
        // However, if we don't have a active identity (which means we're not going to use MLS)
        // we can just use the default path (which creates an empty database).
        let key_store = match active_identity.as_ref() {
            Some(identity) => SledStorage::new_from_path(format!(
                "{}/{}/{}",
                storage_path.to_string_lossy(),
                "mls_storage",
                identity
            )),
            None => SledStorage::new_from_path(format!(
                "{}/{}",
                storage_path.to_string_lossy(),
                "mls_storage"
            )),
        }
        .expect("Failed to create MLS storage with the right path");

        let provider = NostrMlsProvider {
            key_store,
            crypto: RustCrypto::default(),
        };

        Self {
            ciphersuite: Self::DEFAULT_CIPHERSUITE,
            extensions: Self::REQUIRED_EXTENSIONS.to_vec(),
            provider,
        }
    }

    pub fn default_capabilities(&self) -> Capabilities {
        Capabilities::new(
            None,
            Some(&[self.ciphersuite]),
            Some(Self::REQUIRED_EXTENSIONS),
            None,
            None,
        )
    }

    pub fn delete_all_data(&self) -> Result<(), SledStorageError> {
        tracing::debug!(target: "nostr_mls::delete_data", "Deleting all data from key store");
        self.provider.key_store.delete_all_data()
    }

    pub fn ciphersuite_value(&self) -> u16 {
        self.ciphersuite.into()
    }

    pub fn extensions_value(&self) -> String {
        self.extensions
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<String>>()
            .join(",")
    }

    pub fn create_group(
        &self,
        name: String,
        description: String,
        member_key_packages: Vec<KeyPackage>,
        admin_pubkeys_hex: Vec<String>,
        creator_pubkey_hex: String,
        group_relays: Vec<String>,
    ) -> Result<groups::CreateGroupResult, groups::GroupError> {
        groups::create_mls_group(
            self,
            name,
            description,
            member_key_packages,
            admin_pubkeys_hex,
            creator_pubkey_hex,
            group_relays,
        )
    }

    pub fn preview_welcome_event(
        &self,
        welcome_message: Vec<u8>,
    ) -> Result<welcomes::WelcomePreview, welcomes::WelcomeError> {
        welcomes::preview_welcome_event(self, welcome_message)
    }

    pub fn join_group_from_welcome(
        &self,
        welcome_message: Vec<u8>,
    ) -> Result<welcomes::JoinedGroupResult, welcomes::WelcomeError> {
        welcomes::join_group_from_welcome(self, welcome_message)
    }

    pub fn create_message_for_group(
        &self,
        mls_group_id: Vec<u8>,
        message: String,
    ) -> Result<Vec<u8>, groups::GroupError> {
        groups::create_message_for_group(self, mls_group_id, message)
    }

    pub fn export_secret_as_hex_secret_key_and_epoch(
        &self,
        mls_group_id: Vec<u8>,
    ) -> Result<(String, u64), groups::GroupError> {
        groups::export_secret_as_hex_secret_key_and_epoch(self, mls_group_id)
    }

    pub fn process_message_for_group(
        &self,
        mls_group_id: Vec<u8>,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, groups::GroupError> {
        groups::process_message_for_group(self, mls_group_id, message)
    }
}
