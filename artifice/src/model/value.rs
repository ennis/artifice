use crate::model::atom::Atom;
use std::{collections::HashMap, sync::Arc};

pub type Map = HashMap<Atom, Value>;
pub type Array = Vec<Value>;

#[derive(Clone, Data, Debug)]
pub enum Value {
    Number(f64),
    String(Arc<str>),
    Token(Atom),
    Map(Map),
    Array(Array),
    Bool(bool),
    Null,
}

impl Value {
    pub fn as_token(&self) -> Option<&Atom> {
        if let Value::Token(token) = self {
            Some(token)
        } else {
            None
        }
    }

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
