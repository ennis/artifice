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
    fmt::{Formatter, Write},
    hash::{Hash, Hasher},
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

/// A set of methods that a type must implement for it to be storable in a `Value`.
///
/// TODO: StaticValueType
pub trait ValueType: Any + Send + Sync {
    /// Computes the hash of the value.
    fn hash(&self, hasher: &mut dyn Hasher);

    /// Returns the TypeDesc of the stored value, if it can be described by a TypeDesc.
    fn type_desc(&self) -> Option<&TypeDesc>;
}

/// Type-erased value container.
#[derive(Clone)]
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
    //BVec2(BVec2),
    //BVec3(BVec3A),
    //BVec4(BVec4),
    String(Arc<str>),
    Token(Atom),
    Map(Map),
    Array(Array),
    Custom(Arc<dyn ValueType>),
    Null,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(v) => {
                write!(f, "{}i32", v)
            }
            Value::UnsignedInt(v) => {
                write!(f, "{}u32", v)
            }
            Value::Float(v) => {
                write!(f, "{}f32", v)
            }
            Value::Double(v) => {
                write!(f, "{}f64", v)
            }
            Value::Bool(v) => {
                write!(f, "{}", v)
            }
            Value::Vec2(v) => {
                write!(f, "vec2({},{})", v.x, v.y)
            }
            Value::Vec3(v) => {
                write!(f, "vec3({},{},{})", v.x, v.y, v.z)
            }
            Value::Vec4(v) => {
                write!(f, "vec4({},{},{},{})", v.x, v.y, v.z, v.w)
            }
            Value::IVec2(v) => {
                write!(f, "ivec2({},{})", v.x, v.y)
            }
            Value::IVec4(v) => {
                write!(f, "ivec4({},{},{},{})", v.x, v.y, v.z, v.w)
            }
            Value::UVec2(v) => {
                write!(f, "uvec2({},{})", v.x, v.y)
            }
            Value::UVec4(v) => {
                write!(f, "uvec4({},{},{},{})", v.x, v.y, v.z, v.w)
            }
            Value::String(v) => {
                write!(f, "{:?}", v)
            }
            Value::Token(v) => {
                write!(f, "`{}`", v)
            }
            Value::Map(v) => {
                write!(f, "{:?}", v)
            }
            Value::Array(v) => {
                write!(f, "{:?}", v)
            }
            Value::Custom(_) => {
                write!(f, "(custom value)")
                //write!(f, "{:?}", v)
            }
            Value::Null => {
                write!(f, "(null)")
            }
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        todo!()
    }
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
            Value::Custom(v) => v.type_desc().unwrap_or(&TypeDesc::Unknown),
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
