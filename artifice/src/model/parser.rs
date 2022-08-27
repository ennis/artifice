use crate::{
    model,
    model::{
        metadata, typedesc, typedesc::ImageDimension, Document, Node, Param, Path, PrimitiveType, SamplerParameters,
        SamplerWrapMode, TypeDesc, Value,
    },
};
use anyhow::{anyhow, bail};
use artifice::model::typedesc::{ImageType, SampledImageType};
use imbl::OrdMap;
use kyute_common::Atom;
use roxmltree::Children;
use std::{
    fmt,
    fmt::Write,
    io,
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
// Text value parsers
////////////////////////////////////////////////////////////////////////////////////////////////////
fn text_content<'a>(node: roxmltree::Node<'a, '_>) -> Result<&'a str, ReadError> {
    node.first_child()
        .ok_or(ReadError::InvalidValueFormat)?
        .text()
        .ok_or(ReadError::InvalidValueFormat)
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

                    let ty;
                    let value;

                    match ty_name {
                        "float" => {
                            let text = text_content(n)?;
                            value = Value::Float(text.parse()?);
                            ty = TypeDesc::FLOAT;
                        }
                        "double" => {
                            let text = text_content(n)?;
                            value = Value::Double(text.parse()?);
                            ty = TypeDesc::DOUBLE;
                        }
                        "vec2" | "float2" => {
                            let text = text_content(n)?;
                            value = Value::Vec2(parse_vec2(text)?);
                            ty = TypeDesc::VEC2;
                        }
                        "vec3" | "float3" => {
                            let text = text_content(n)?;
                            value = Value::Vec3(parse_vec3(text)?);
                            ty = TypeDesc::VEC3;
                        }
                        "vec4" | "float4" => {
                            let text = text_content(n)?;
                            value = Value::Vec4(parse_vec4(text)?);
                            ty = TypeDesc::VEC4;
                        }
                        "string" => {
                            value = Value::String(text_content(n)?.into());
                            ty = TypeDesc::String;
                        }
                        "int" => {
                            let text = text_content(n)?;
                            value = Value::Int(text.parse()?);
                            ty = TypeDesc::INT;
                        }
                        "uint" => {
                            let text = text_content(n)?;
                            value = Value::UnsignedInt(text.parse()?);
                            ty = TypeDesc::UNSIGNED_INT;
                        }
                        "bool" => {
                            let text = text_content(n)?;
                            value = Value::Bool(text.parse()?);
                            ty = TypeDesc::BOOL;
                        }
                        "sampler" => {
                            let mut sampler = SamplerParameters::default();

                            for child in n.children() {
                                if !child.is_element() {
                                    warn!("unexpected data in `<sampler>` element");
                                    continue;
                                }
                                match child.tag_name().name() {
                                    "wrapModeS" => {
                                        sampler.wrap_mode_s = parse_wrap_mode(text_content(child)?)?;
                                    }
                                    "wrapModeT" => {
                                        sampler.wrap_mode_t = parse_wrap_mode(text_content(child)?)?;
                                    }
                                    "wrapModeR" => {
                                        sampler.wrap_mode_r = parse_wrap_mode(text_content(child)?)?;
                                    }
                                    other => {
                                        warn!("unexpected element `<{other}>`");
                                    }
                                }
                            }

                            value = Value::Custom(Arc::new(sampler));
                            ty = TypeDesc::SAMPLER;
                        }
                        "texture1D" | "texture2D" | "texture3D" => {
                            // Textures are not serializable, so they don't have values in the XML document.
                            // They usually are outputs of nodes.
                            value = Value::Null;
                            ty = TypeDesc::SampledImage(Arc::new(SampledImageType {
                                sampled_ty: PrimitiveType::Float,
                                dim: match ty_name {
                                    "texture1D" => ImageDimension::Dim1D,
                                    "texture2D" => ImageDimension::Dim2D,
                                    "texture3D" => ImageDimension::Dim3D,
                                    _ => unreachable!(),
                                },
                                ms: false,
                            }));
                        }
                        _ => {
                            warn!("unknown value type: `<{ty_name}>`");
                            value = Value::Null;
                            ty = TypeDesc::Unknown;
                        }
                    }

                    params.insert(
                        param_name.clone(),
                        Param {
                            rev: 0,
                            id: 0,
                            path: node_path.join_attribute(param_name),
                            ty,
                            value: Some(value),
                            connection,
                            metadata: Default::default(),
                        },
                    );
                }
            }
        }

        let mut metadata = OrdMap::new();
        if !op.is_empty() {
            metadata.insert(Atom::from(metadata::OPERATOR.name), Value::from(op));
        }

        Ok(Node {
            rev: 0,
            id: 0,
            path: node_path,
            attributes: params,
            metadata,
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
