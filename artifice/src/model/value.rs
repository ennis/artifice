use crate::model::{atom::Atom, Value::Array};
use kyute::Data;
use serde::{
    de::{EnumAccess, Error, MapAccess, SeqAccess, Unexpected},
    ser::{SerializeMap, SerializeSeq},
    Deserializer, Serializer,
};
use std::{collections::HashMap, convert::TryInto, fmt::Formatter, sync::Arc};

pub type Map = imbl::HashMap<Atom, Value>;
pub type Array = imbl::Vector<Value>;

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

impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Number(num) => serializer.serialize_f64(num),
            Value::String(ref str) => serializer.serialize_str(str),
            Value::Token(ref atom) => serializer.serialize_str(atom),
            Value::Map(ref map) => {
                let mut ser_map = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map.iter() {
                    ser_map.serialize_entry(k, v);
                }
                ser_map.end()
            }
            Value::Array(ref array) => {
                let mut ser_array = serializer.serialize_seq(Some(array.len()))?;
                for item in array.iter() {
                    ser_array.serialize_element(item);
                }
                ser_array.end()
            }
            Value::Bool(val) => serializer.serialize_bool(val),
            Value::Null => serializer.serialize_none(),
        }
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "any numeric type or string, bytes, map, array, none"
        )
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Bool(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        //let v = v.try_into().map_err(|err| E::custom(err))?;
        Ok(Value::Number(v as f64))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Number(v))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Err(E::invalid_type(Unexpected::Char(v), &self))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v.into()))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v.into()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v.into()))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        todo!()
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        todo!()
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        todo!()
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        todo!()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut array = Array::new();
        while let Some(elem) = access.next_element()? {
            array.push_back(elem);
        }
        Ok(Value::Array(array))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut map = Map::new();
        while let Some((key, value)) = access.next_entry()? {
            map.insert(key, value);
        }
        Ok(Value::Map(map))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        todo!()
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}
