use crate::{
    model,
    model::{
        typedesc, Document, Node, Param, Path, PrimitiveType, SamplerParameters, SamplerWrapMode, TypeDesc, Value,
    },
};
use anyhow::{anyhow, bail};
use imbl::OrdMap;
use kyute_common::Atom;
use std::{
    fmt, io,
    io::BufRead,
    num::{ParseFloatError, ParseIntError},
    str::{FromStr, ParseBoolError},
    sync::Arc,
};
use thiserror::Error;
use tracing::warn;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Error
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Error)]
pub enum ReadError {
    #[error("parse error")]
    ParseError(#[from] roxmltree::Error),
    #[error("unexpected element")]
    UnexpectedElement { tag: String },
    #[error("no <document> element found")]
    MissingDocumentElement,
    #[error("more than one <document> element found")]
    TooManyDocuments,
    #[error("missing attribute")]
    MissingAttribute,
    #[error("non UTF-8 name")]
    NonUtf8Name,
    #[error("integer parse error")]
    ParseIntError(#[from] ParseIntError),
    #[error("float parse error")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("boolean error")]
    ParseBoolError(#[from] ParseBoolError),
    #[error("invalid value format")]
    InvalidValueFormat,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Type registration
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Represents a factory object for reading and writing values of a specific type to an XML file.
pub trait ValueIo {
    /// The type of the values read.
    fn type_desc(&self) -> TypeDesc;
    /// Read a value from a sequence of XML nodes.
    fn read(&self, nodes: roxmltree::Children) -> Result<Value, ReadError>;
    fn write(&self, value: &Value, writer: &mut dyn fmt::Write);
}

///
pub struct ValueIoRegistration {
    name: &'static str,
    methods: &'static (dyn ValueIo + Send + Sync),
}

impl ValueIoRegistration {
    pub const fn new(name: &'static str, methods: &'static (dyn ValueIo + Send + Sync)) -> ValueIoRegistration {
        ValueIoRegistration { name, methods }
    }

    /// Returns registered IO methods for the specified type name.
    fn find(name: &str) -> Option<&'static ValueIoRegistration> {
        for reg in inventory::iter::<ValueIoRegistration> {
            if reg.name == name {
                return Some(reg);
            }
        }
        return None;
    }
}

pub(crate) fn get_value_io_api(name: &str) -> Option<&'static (dyn ValueIo + Send + Sync)> {
    ValueIoRegistration::find(name).map(|r| r.methods)
}

inventory::collect!(ValueIoRegistration);

////////////////////////////////////////////////////////////////////////////////////////////////////
// Text value parsers
////////////////////////////////////////////////////////////////////////////////////////////////////
fn element_content_as_text<'a>(elem: roxmltree::Node<'a, '_>) -> &'a str {
    // TODO errors
    elem.first_child().unwrap().text().unwrap()
}

/*fn parse_primitive_value(text: &str, ty: PrimitiveType) -> Result<Value, ReadError> {
    match ty {
        PrimitiveType::Int => {
            let v: i32 = text.parse()?;
            Ok(Value::Int(v))
        }
        PrimitiveType::UnsignedInt => {
            let v: u32 = text.parse()?;
            Ok(Value::UnsignedInt(v))
        }
        PrimitiveType::Float => {
            let v: f32 = text.parse()?;
            Ok(Value::Float(v))
        }
        PrimitiveType::Double => {
            let v: f64 = text.parse()?;
            Ok(Value::Double(v))
        }
        PrimitiveType::Bool => {
            let v: bool = text.parse()?;
            Ok(Value::Bool(v))
        }
    }
}*/

/// Implementation of ValueIo for primitive types
macro_rules! impl_simple_value_io {
    ($base_ty:ty, $value_variant:ident, $tydesc:expr, $tyname:literal) => {
        struct $value_variant;
        impl ValueIo for $value_variant {
            fn type_desc(&self) -> TypeDesc {
                $tydesc.clone()
            }

            fn read(&self, mut nodes: roxmltree::Children) -> Result<Value, ReadError> {
                let text = nodes
                    .next()
                    .ok_or(ReadError::InvalidValueFormat)?
                    .text()
                    .ok_or(ReadError::InvalidValueFormat)?;
                let value: $base_ty = text.parse()?;
                Ok(Value::$value_variant(value))
            }

            fn write(&self, _value: &Value, _writer: &mut dyn fmt::Write) {
                todo!()
            }
        }
        inventory::submit! {
            ValueIoRegistration::new($tyname, &$value_variant)
        }
    };
}

impl_simple_value_io!(f32, Float, TypeDesc::FLOAT, "float");
impl_simple_value_io!(f64, Double, TypeDesc::DOUBLE, "double");
impl_simple_value_io!(i32, Int, TypeDesc::INT, "int");
impl_simple_value_io!(u32, UnsignedInt, TypeDesc::UNSIGNED_INT, "uint");
impl_simple_value_io!(bool, Bool, TypeDesc::BOOL, "bool");

///
fn parse_array<T: FromStr>(text: &str, expected: Option<usize>) -> Result<Vec<T>, ReadError>
where
    ReadError: From<<T as FromStr>::Err>,
{
    let mut result = Vec::with_capacity(expected.unwrap_or(0));
    for elem in text.split(',') {
        result.push(elem.parse::<T>()?);
    }
    if let Some(len) = expected {
        if len != result.len() {
            return Err(ReadError::InvalidValueFormat);
        }
    }
    Ok(result)
}

/// Parsing functions for vector value representations.
///
/// Vectors are represented in XML as n comma-separated values, possibly with whitespace between the values,
/// where n is exactly the number of vector components (e.g. "0.0, 1.0, 0.0, 0.5" for a Vec4).
macro_rules! impl_parse_vector {
    ($vec_ty:ty, $elem_ty:ty, $len:literal, $parse_fn:ident) => {
        fn $parse_fn(text: &str) -> Result<$vec_ty, ReadError> {
            let mut out = <$vec_ty>::default();
            let mut n_comp = 0;
            for comp in text.split(',') {
                if n_comp >= $len {
                    return Err(ReadError::InvalidValueFormat);
                }
                out[n_comp] = comp.trim().parse::<$elem_ty>()?;
                n_comp += 1;
            }
            if n_comp != $len {
                return Err(ReadError::InvalidValueFormat);
            }
            Ok(out)
        }
    };
}

impl_parse_vector!(glam::Vec2, f32, 2, parse_vec2);
impl_parse_vector!(glam::Vec3A, f32, 3, parse_vec3);
impl_parse_vector!(glam::Vec4, f32, 4, parse_vec4);
impl_parse_vector!(glam::IVec2, i32, 2, parse_ivec2);
//impl_parse_vector!(glam::IVec3A, i32, 3, parse_ivec3);
impl_parse_vector!(glam::IVec4, i32, 4, parse_ivec4);
impl_parse_vector!(glam::UVec2, u32, 2, parse_uvec2);
//impl_parse_vector!(glam::UVec3A, u32, 3, parse_uvec3);
impl_parse_vector!(glam::UVec4, u32, 4, parse_uvec4);

//impl_parse_vector!(glam::BVec2, bool, 2, parse_bvec2);
//impl_parse_vector!(glam::BVec3A, bool, 3, parse_bvec3);
//impl_parse_vector!(glam::BVec4, bool, 4, parse_bvec4);

/// Implementation of ValueIo for vector types
macro_rules! impl_vector_value_io {
    ($base_ty:ty, $value_variant:ident, $tydesc:expr, $tyname:literal, $parsefn:ident) => {
        struct $value_variant;
        impl ValueIo for $value_variant {
            fn type_desc(&self) -> TypeDesc {
                $tydesc.clone()
            }
            fn read(&self, mut nodes: roxmltree::Children) -> Result<Value, ReadError> {
                let text = nodes
                    .next()
                    .ok_or(ReadError::InvalidValueFormat)?
                    .text()
                    .ok_or(ReadError::InvalidValueFormat)?;
                let value: $base_ty = $parsefn(text)?;
                Ok(Value::$value_variant(value))
            }

            fn write(&self, value: &Value, writer: &mut dyn fmt::Write) {
                todo!()
            }
        }
        inventory::submit! {
            ValueIoRegistration::new($tyname, &$value_variant)
        }
    };
}

impl_vector_value_io!(glam::Vec2, Vec2, TypeDesc::VEC2, "vec2", parse_vec2);
impl_vector_value_io!(glam::Vec3A, Vec3, TypeDesc::VEC3, "vec3", parse_vec3);
impl_vector_value_io!(glam::Vec4, Vec4, TypeDesc::VEC4, "vec4", parse_vec4);
impl_vector_value_io!(glam::IVec2, IVec2, TypeDesc::IVEC2, "ivec2", parse_ivec2);
//impl_vector_value_io!(glam::IVec3, IVec3, "ivec3", parse_ivec3);
impl_vector_value_io!(glam::IVec4, IVec4, TypeDesc::IVEC4, "ivec4", parse_ivec4);
impl_vector_value_io!(glam::UVec2, UVec2, TypeDesc::UVEC2, "uvec2", parse_uvec2);
//impl_vector_value_io!(glam::UVec3, UVec3, "uvec3", parse_ivec3);
impl_vector_value_io!(glam::UVec4, UVec4, TypeDesc::UVEC4, "uvec4", parse_uvec4);
//impl_vector_value_io!(glam::BVec2, BVec2, "bvec2", parse_bvec2);
//impl_vector_value_io!(glam::BVec3A, BVec3, "bvec3", parse_bvec3);
//impl_vector_value_io!(glam::BVec4, BVec4, "bvec4", parse_bvec4);

////////////////////////////////////////////////////////////////////////////////////////////////////
// Samplers
////////////////////////////////////////////////////////////////////////////////////////////////////
fn parse_wrap_mode(text: &str) -> Result<SamplerWrapMode, ReadError> {
    match text {
        "clamp" => Ok(SamplerWrapMode::Clamp),
        "repeat" => Ok(SamplerWrapMode::Repeat),
        "mirror" => Ok(SamplerWrapMode::Mirror),
        other => {
            error!("invalid sampler wrap mode: `{}`", other);
            Err(ReadError::InvalidValueFormat)
        }
    }
}

struct SamplerParameterIo;

impl ValueIo for SamplerParameterIo {
    fn type_desc(&self) -> TypeDesc {
        TypeDesc::Sampler
    }

    fn read(&self, nodes: roxmltree::Children) -> Result<Value, ReadError> {
        let mut sampler = SamplerParameters::default();

        for node in nodes {
            match node.tag_name().name() {
                "wrapModeS" => {
                    sampler.wrap_mode_s = parse_wrap_mode(element_content_as_text(node))?;
                }
                "wrapModeT" => {
                    sampler.wrap_mode_t = parse_wrap_mode(element_content_as_text(node))?;
                }
                "wrapModeR" => {
                    sampler.wrap_mode_r = parse_wrap_mode(element_content_as_text(node))?;
                }
                _ => {}
            }
        }

        Ok(Value::Custom(Arc::new(sampler)))
    }

    fn write(&self, value: &Value, writer: &mut dyn fmt::Write) {
        todo!()
    }
}

inventory::submit! {
    ValueIoRegistration::new("sampler", &SamplerParameterIo)
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Nodes
////////////////////////////////////////////////////////////////////////////////////////////////////

impl Param {}

impl Node {
    fn read(parent_path: Path, xml_node: roxmltree::Node) -> Result<Node, ReadError> {
        let mut name = Atom::default();
        let mut op = Atom::default();

        let tag_name = xml_node.tag_name().name();
        assert_eq!(tag_name, "node");
        for attr in xml_node.attributes() {
            match attr.name() {
                "id" => {
                    name = attr.value().into();
                }
                "op" => {
                    op = attr.value().into();
                }
                _ => {
                    warn!("unrecognized node attribute: {}=\"{}\"", attr.name(), attr.value());
                }
            }
        }

        let node_path = parent_path.join(name.clone());
        //let mut ports = vec![];
        let mut params = OrdMap::new();
        let mut children = OrdMap::new();

        for n in xml_node.children() {
            if !n.is_element() {
                continue;
            }
            match n.tag_name().name() {
                "port" => {
                    // TODO read ports
                }
                "node" => {
                    let child = Node::read(node_path.clone(), n)?;
                    children.insert(child.name(), child);
                }
                ty_name => {
                    // this is a parameter
                    let mut param_name = Atom::default();
                    let mut connection = None;
                    for attr in n.attributes() {
                        match attr.name() {
                            "id" => {
                                param_name = attr.value().into();
                            }
                            "connect" => {
                                connection = Some(Path::parse(attr.value()).unwrap());
                            }
                            _ => {
                                warn!("unrecognized param attribute: {}=\"{}\"", attr.name(), attr.value());
                            }
                        }
                    }

                    // read the value if there's one
                    let (ty, value) = if let Some(io) = get_value_io_api(ty_name) {
                        let ty = io.type_desc();
                        let value = if n.has_children() {
                            Some(io.read(n.children())?)
                        } else {
                            None
                        };
                        (ty, value)
                    } else {
                        warn!("unknown value type: `<{ty_name}>`");
                        (TypeDesc::Unknown, None)
                    };

                    params.insert(
                        param_name.clone(),
                        Param {
                            rev: 0,
                            id: 0,
                            path: node_path.join_attribute(param_name),
                            ty,
                            value,
                            connection,
                            metadata: Default::default(),
                        },
                    );
                }
            }
        }

        Ok(Node {
            rev: 0,
            id: 0,
            path: node_path,
            attributes: params,
            metadata: Default::default(),
            children,
        })
    }
}

impl Document {
    pub fn from_xml(xml: &str) -> Result<Document, ReadError> {
        let xml = roxmltree::Document::parse(xml)?;
        let mut seen_document = false;
        let mut root = Node::new(0, Path::root());

        for child in xml.root().children() {
            if !child.is_element() {
                continue;
            }
            match child.tag_name().name() {
                "document" => {
                    if seen_document {
                        return Err(ReadError::TooManyDocuments);
                    }
                    seen_document = true;

                    // load root nodes
                    for node in child.children() {
                        if !node.is_element() {
                            continue;
                        }
                        match node.tag_name().name() {
                            "node" => {
                                let n = Node::read(root.path.clone(), node)?;
                                println!("{:?}", n);
                                root.children.insert(n.name(), n);
                            }
                            other => {
                                warn!("unknown element tag: `<{}>`", other)
                            }
                        }
                    }
                }
                other => {
                    warn!("unknown element: `<{}>`", other)
                }
            }
        }

        if !seen_document {
            //error!("no `<document>` element found");
            return Err(ReadError::MissingDocumentElement);
        }

        Ok(Document { revision: 0, root })
    }
}

#[cfg(test)]
mod tests {}
