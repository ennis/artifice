//! Application data model
use druid::{ArcStr, Data, FileInfo, Lens};
use std::{fs::File, path::Path, sync::Arc};

use crate::data::{atom::Atom, network::Network, node::Node};
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

pub type Map = druid::im::HashMap<Atom, Value>;
pub type Array = druid::im::Vector<Value>;

#[derive(Clone, Data, Debug)]
pub enum Value {
    Number(f64),
    String(druid::ArcStr),
    Token(Atom),
    Map(Map),
    Array(Array),
    Bool(bool),
    Null,
}

impl Value {
    /// Returns a reference to the string, if this is a string value.
    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(str) = self {
            Some(str)
        } else {
            None
        }
    }

    /// Extracts the number if this object contains one.
    pub fn as_number(&self) -> Option<f64> {
        if let Value::Number(num) = self {
            Some(*num)
        } else {
            None
        }
    }

    /// Extracts the boolean value if this object contains one.
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(bool) = self {
            Some(*bool)
        } else {
            None
        }
    }

    pub fn as_map(&self) -> Option<&Map> {
        if let Value::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        if let Value::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&Array> {
        if let Value::Array(array) = self {
            Some(array)
        } else {
            None
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        if let Value::Array(array) = self {
            Some(array)
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Number(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Number(v as f64)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v.into())
    }
}

/// Lens to view the contents of a generic value as a number (`Value::number`).
pub struct AsNumberLens;
impl Lens<Value, f64> for AsNumberLens {
    fn with<V, F: FnOnce(&f64) -> V>(&self, data: &Value, f: F) -> V {
        let data_f64 = data.as_number().unwrap_or_default();
        f(&data_f64)
    }

    fn with_mut<V, F: FnOnce(&mut f64) -> V>(&self, data: &mut Value, f: F) -> V {
        let mut data_f64 = data.as_number().unwrap_or_default();
        let v = f(&mut data_f64);
        *data = Value::from(data_f64);
        v
    }
}

pub struct AsIntegerLens;
impl Lens<Value, i32> for AsIntegerLens {
    fn with<V, F: FnOnce(&i32) -> V>(&self, data: &Value, f: F) -> V {
        let data_i32 = data.as_number().unwrap_or_default() as i32;
        f(&data_i32)
    }

    fn with_mut<V, F: FnOnce(&mut i32) -> V>(&self, data: &mut Value, f: F) -> V {
        let mut data_i32 = data.as_number().unwrap_or_default() as i32;
        let v = f(&mut data_i32);
        *data = Value::from(data_i32);
        v
    }
}

pub struct AsBoolLens;
impl Lens<Value, bool> for AsBoolLens {
    fn with<V, F: FnOnce(&bool) -> V>(&self, data: &Value, f: F) -> V {
        let data_bool = data.as_bool().unwrap_or_default();
        f(&data_bool)
    }

    fn with_mut<V, F: FnOnce(&mut bool) -> V>(&self, data: &mut Value, f: F) -> V {
        let mut data_bool = data.as_bool().unwrap_or_default();
        let v = f(&mut data_bool);
        *data = Value::from(data_bool);
        v
    }
}

pub struct AsStringLens;
impl Lens<Value, String> for AsStringLens {
    fn with<V, F: FnOnce(&String) -> V>(&self, data: &Value, f: F) -> V {
        let data_string = data.as_str().unwrap().to_owned();
        f(&data_string)
    }

    fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, data: &mut Value, f: F) -> V {
        let mut data_string = data.as_str().unwrap().to_owned();
        let v = f(&mut data_string);
        *data = Value::from(data_string);
        v
    }
}
