//! Application data model
use std::{collections::HashMap, fs::File, path::Path, sync::Arc};
use crate::model::{atom::Atom, network::Network, node::Node};
use anyhow::Context;
use kyute::Data;
use lazy_static::lazy_static;
use crate::json;
use crate::model::path::ModelPath;
use crate::model::value::Value;

pub mod atom;
pub mod network;
pub mod node;
pub mod property;
mod value;
mod path;
mod document;

lazy_static! {
    pub static ref MODEL_ATOMS: ModelAtoms = ModelAtoms::new();
}

pub struct ModelAtoms {
    pub name: Atom,
    pub children: Atom,
}

impl ModelAtoms {
    fn new() -> Self {
        ModelAtoms {
            name: Atom::from("name"),
            children: Atom::from("children")
        }
    }
}

/// Objects that belong to the document object tree, they have a name and are accessible via their path.
#[derive(Clone,Data)]
pub struct NamedObject {
    name: Atom,
}

impl Data for NamedObject {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}

impl NamedObject {

    pub fn from_json(value: &json::Value) {

    }

    /// Returns the name of this object.
    pub fn name(&self) -> &Atom {
        &self.name
    }
}

/// Value share group.
#[derive(Clone,Data)]
pub struct ShareGroup {
    shares: Arc<Vec<ModelPath>>,
    value: Value,
}


/*
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
}*/
