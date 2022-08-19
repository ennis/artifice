use crate::model::{Atom, PrimitiveType, TypeDesc};
use kyute::Data;
use serde::{
    de::{EnumAccess, Error, MapAccess, SeqAccess, Unexpected},
    ser::{SerializeMap, SerializeSeq},
    Deserializer, Serializer,
};
use std::{
    any::Any,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt,
    fmt::Write,
    io,
    sync::Arc,
};
use thiserror::Error;

pub type Map = imbl::HashMap<Atom, Value>;
pub type Array = imbl::Vector<Value>;

pub type Vec2 = glam::Vec2;
pub type Vec3 = glam::Vec3;
pub type Vec3A = glam::Vec3A;
pub type Vec4 = glam::Vec4;

pub type IVec2 = glam::IVec2;
pub type IVec3 = glam::IVec3;
//pub type IVec3A = glam::IVec3A;
pub type IVec4 = glam::IVec4;

pub type UVec2 = glam::UVec2;
pub type UVec3 = glam::UVec3;
//pub type UVec3A = glam::UVec3A;
pub type UVec4 = glam::UVec4;

pub type BVec2 = glam::BVec2;
pub type BVec3 = glam::BVec3;
pub type BVec3A = glam::BVec3A;
pub type BVec4 = glam::BVec4;

/// Trait for types that have an associated `TypeDesc`.
pub trait HasTypeDesc {
    fn type_desc() -> TypeDesc;
}

macro_rules! impl_has_type_desc_primitive {
    ($t:ty, $prim:ident) => {
        impl HasTypeDesc for $t {
            fn type_desc() -> TypeDesc {
                TypeDesc::Primitive(PrimitiveType::$prim)
            }
        }
    };
}

impl_has_type_desc_primitive!(i32, Int);
impl_has_type_desc_primitive!(u32, UnsignedInt);
impl_has_type_desc_primitive!(f32, Float);
impl_has_type_desc_primitive!(f64, Double);
impl_has_type_desc_primitive!(bool, Bool);

/// Type-erased value containers.
#[derive(Clone, Debug)]
pub enum Value {
    // Store small values directly as variants of the enum. For more complex types,
    // defer to Custom.
    Int(i32),
    UnsignedInt(u32),
    Float(f32),
    Double(f64),
    Bool(bool),
    Vec2(Vec2),
    Vec3(Vec3A),
    Vec4(Vec4),
    IVec2(IVec2),
    //IVec3(IVec3A),
    IVec4(IVec4),
    UVec2(UVec2),
    //UVec3(UVec3A),
    UVec4(UVec4),
    /*BVec2(BVec2),
    BVec3(BVec3A),
    BVec4(BVec4),*/
    String(Arc<str>),
    Token(Atom),
    Map(Map),
    Array(Array),
    Custom {
        type_desc: Option<TypeDesc>,
        data: Arc<dyn Any + Send + Sync>,
    },
    Null,
}

impl Value {
    /// Returns the type descriptor of the value contained inside this object.
    pub fn type_desc(&self) -> &TypeDesc {
        match self {
            Value::Int(_) => &TypeDesc::INT,
            Value::UnsignedInt(_) => &TypeDesc::UNSIGNED_INT,
            Value::Float(_) => &TypeDesc::FLOAT,
            Value::Double(_) => &TypeDesc::DOUBLE,
            Value::Bool(_) => &TypeDesc::BOOL,
            Value::Vec2(_) => &TypeDesc::VEC2,
            Value::Vec3(_) => &TypeDesc::VEC3,
            Value::Vec4(_) => &TypeDesc::VEC4,
            Value::String(_) => &TypeDesc::String,
            Value::Token(_) => {
                todo!()
            }
            Value::Map(_) => {
                todo!()
            }
            Value::Array(_) => {
                todo!()
            }
            Value::Custom { ref type_desc, .. } => type_desc.as_ref().unwrap_or(&TypeDesc::Unknown),
            Value::Null => &TypeDesc::Void,
            Value::IVec2(_) => &TypeDesc::IVEC2,
            Value::IVec4(_) => &TypeDesc::IVEC3,
            Value::UVec2(_) => &TypeDesc::UVEC2,
            Value::UVec4(_) => &TypeDesc::UVEC4,
            /*Value::BVec2(_) => &TypeDesc::
            Value::BVec3(_) => {}
            Value::BVec4(_) => {}*/
        }
    }

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
    pub fn as_double(&self) -> Option<f64> {
        if let Value::Double(num) = self {
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
        Value::Double(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
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

impl<'a> From<&'a str> for Value {
    fn from(v: &'a str) -> Self {
        Value::String(v.into())
    }
}

/*impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Value::Int(num) => serializer.serialize_f64(num),
            Value::String(ref str) => serializer.serialize_str(str),
            Value::Token(ref atom) => serializer.serialize_str(atom),
            Value::Map(ref map) => {
                let mut ser_map = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map.iter() {
                    ser_map.serialize_entry(k, v)?;
                }
                ser_map.end()
            }
            Value::Array(ref array) => {
                let mut ser_array = serializer.serialize_seq(Some(array.len()))?;
                for item in array.iter() {
                    ser_array.serialize_element(item)?;
                }
                ser_array.end()
            }
            Value::Bool(val) => serializer.serialize_bool(val),
            Value::Null => serializer.serialize_none(),
        }
    }
}*/

/*struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "any numeric type or string, bytes, map, array, none")
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
        deserializer.deserialize_any(ValueVisitor)
    }
}
*/

////////////////////////////////////////////////////////////////////////////////////////////////////
// FromValue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The error type return when a `Value` type conversion fails.
#[derive(Debug, Error)]
#[error("failed to convert value to target type")]
pub struct TryFromValueError;

impl TryFrom<Value> for f64 {
    type Error = TryFromValueError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int(v) => Ok(v as f64),
            Value::UnsignedInt(v) => Ok(v as f64),
            Value::Float(v) => Ok(v as f64),
            Value::Double(v) => Ok(v),
            _ => Err(TryFromValueError),
        }
    }
}

impl TryFrom<Value> for Atom {
    type Error = TryFromValueError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Token(v) => Ok(v),
            Value::String(v) => Ok(Atom::from(&*v)),
            _ => Err(TryFromValueError),
        }
    }
}

impl TryFrom<Value> for String {
    type Error = TryFromValueError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Token(v) => Ok(v.to_string()),
            Value::String(v) => Ok(v.to_string()),
            _ => Err(TryFromValueError),
        }
    }
}
