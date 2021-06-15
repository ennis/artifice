//! Application data model
use druid::{Data, FileInfo, Lens};
use std::{fs::File, path::Path, sync::Arc};

use crate::data::{network::Network, node::Node};
use anyhow::Context;

pub mod atom;
pub mod network;
pub mod node;
pub mod property;

/// Application state
#[derive(Clone, Data)]
pub struct AppData {
    /// Network being edited
    pub network: Network,

    /// Path to the file being edited, empty if not saved yet
    #[data(ignore)]
    pub current_file_info: Option<FileInfo>,
}

impl AppData {
    /// Creates a new, blank application state.
    pub fn new() -> AppData {
        AppData {
            network: Network::new(),
            current_file_info: None,
        }
    }

    /// Saves the current network to the specified file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let mut file = File::create(path).context("Failed to open file for saving")?;
        let json = self.network.to_json();
        serde_json::to_writer_pretty(&mut file, &json).context("failed to write JSON")?;
        Ok(())
    }

    /// Loads the network from the specified file.
    /// If it fails, the current network is left unchanged.
    pub fn load(&mut self, path: &Path) -> anyhow::Result<()> {
        let file = File::open(path).context("Failed to open file for reading")?;
        let json: serde_json::Value = serde_json::from_reader(file).context("invalid JSON")?;
        self.network = Network::from_json(&json)?;
        Ok(())
    }

    /// Returns a lens to the network.
    pub fn network_lens() -> impl Lens<Self, Network> {
        druid::lens!(Self, network)
    }
}
