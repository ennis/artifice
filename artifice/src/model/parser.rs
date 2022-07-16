use crate::model::{typedesc, Document, Node, Param, Path, PrimitiveType, TypeDesc, Value};
use std::{
    io,
    io::BufRead,
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
    sync::Arc,
};
use thiserror::Error;
use tracing::warn;
use xmlparser::{ElementEnd, Token};

pub struct Reader<'a, 'b> {
    tokenizer: &'b mut xmlparser::Tokenizer<'a>,
    empty: bool,
}

impl<'a, 'b> Reader<'a, 'b> {
    fn consume_end_tag(&mut self) -> Result<(), ReadError> {
        match self.tokenizer.next() {
            Token::ElementEnd {
                end: ElementEnd::Close(_, _),
                ..
            } => Ok(()),
            _ => Err(ReadError::UnexpectedElement),
        }
    }

    pub fn parse_elements(&mut self, f: impl FnMut(&str, AttributeReader)) {
        if self.empty {
            return;
        }

        loop {
            match self.tokenizer.next() {
                Token::ElementStart { local, .. } => {
                    let attr_reader = AttributeReader {
                        tokenizer: self.tokenizer,
                    };
                    f(local.as_str(), attr_reader);
                }
                _ => {
                    panic!("unexpected token")
                }
                Token::ElementEnd { end, .. } => match end {
                    ElementEnd::Close(_, _) => break,
                    _ => panic!("unexpected element end token"),
                },
            }
        }
    }

    pub fn expect_text(&mut self) -> Result<&'a str, ReadError> {
        let text = match self.tokenizer.next() {
            Token::Text { text } => Ok(text.as_str()),
            Token::Cdata { text, .. } => Ok(text.as_str()),
            _ => Err(ReadError::UnexpectedElement),
        }?;
        self.consume_end_tag()?;
        Ok(text)
    }
}

pub struct AttributeReader<'a, 'b> {
    tokenizer: &'b mut xmlparser::Tokenizer<'a>,
}

impl<'a, 'b> AttributeReader<'a, 'b> {
    pub fn skip_attributes(mut self) -> Reader<'a, 'b> {
        loop {
            match self.tokenizer.next() {
                Token::ElementEnd { end, .. } => match end {
                    ElementEnd::Empty => {
                        return Reader {
                            tokenizer: self.tokenizer,
                            empty: true,
                        }
                    }
                    ElementEnd::Open => {
                        return Reader {
                            tokenizer: self.tokenizer,
                            empty: false,
                        }
                    }
                    _ => panic!("unexpected token"),
                },
                _ => {}
            }
        }
    }

    pub fn parse_attributes(mut self, f: impl FnMut(&str, &str)) -> Reader<'a, 'b> {
        loop {
            match self.tokenizer.next() {
                Token::Attribute { local, value, .. } => f(local.as_str(), value.as_str()),
                Token::ElementEnd { end, .. } => match end {
                    ElementEnd::Empty => {
                        return Reader {
                            tokenizer: self.tokenizer,
                            empty: true,
                        }
                    }
                    ElementEnd::Open => {
                        return Reader {
                            tokenizer: self.tokenizer,
                            empty: false,
                        }
                    }
                    _ => panic!("unexpected token"),
                },
                _ => panic!("unexpected token"),
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Type registration
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Methods for reading and writing values to an XML file.
pub trait ValueType {
    fn type_desc(&self) -> TypeDesc;
    fn read(&self, reader: &mut Reader) -> anyhow::Result<Value>;
    fn write(&self, value: &Value, writer: &mut XmlWriter);
}

///
pub struct ValueTypeRegistration {
    name: String,
    methods: &'static dyn ValueType,
}

impl ValueTypeRegistration {
    pub fn new(name: impl Into<String>, methods: &'static dyn ValueType) -> ValueTypeRegistration {
        XmlValueTypeRegistration {
            name: name.into(),
            methods,
        }
    }
}

inventory::collect!(ValueTypeRegistration);

macro_rules! impl_primitive_value_type {
    ($base_ty:ty, $value_variant:ident, $tyname:literal) => {
        struct $value_variant;
        impl ValueType for $value_variant {
            fn type_desc(&self) -> TypeDesc {
                TypeDesc::Primitive(PrimitiveType::$value_variant)
            }

            fn read(&self, reader: &mut Reader) -> anyhow::Result<Value> {
                let text = reader.expect_text()?;
                reader.expect_end_element();
                let value: $base_ty = text.parse()?;
                Ok(Value::$value_variant(value))
            }

            fn write(&self, value: &Value, writer: &mut Writer) {
                todo!()
            }
        }
        inventory::submit! {
            ValueTypeRegistration::new($tyname, &$value_variant)
        }
    };
}

impl_primitive_value_type!(f32, Float, "float");
impl_primitive_value_type!(f64, Double, "double");
impl_primitive_value_type!(i32, Int, "int");
impl_primitive_value_type!(u32, UnsignedInt, "uint");
impl_primitive_value_type!(bool, Bool, "bool");

fn parse_wrap_mode(text: &str) -> Result<SamplerWrapMode, ReadError> {
    match text {
        "clamp" => Ok(SamplerWrapMode::Clamp),
        "repeat" => Ok(SamplerWrapMode::Repeat),
        "mirror" => Ok(SamplerWrapMode::Mirror),
        _ => Err(ReadError::UnexpectedToken),
    }
}

struct SamplerType;
impl ValueType for SamplerType {
    fn type_desc(&self) -> TypeDesc {
        TypeDesc::Sampler
    }

    fn read(&self, reader: &mut Reader) -> anyhow::Result<Value> {
        let mut wrap_mode_s = SamplerWrapMode::Clamp;
        let mut wrap_mode_t = SamplerWrapMode::Clamp;
        let mut wrap_mode_r = SamplerWrapMode::Clamp;

        loop {
            reader.parse_elements(|name, reader| {
                let mut reader = reader.skip_attributes();
                match name {
                    "wrapModeS" => {
                        let value = reader.expect_text()?;
                        wrap_mode_s = parse_wrap_mode(value)?;
                    }
                    "wrapModeT" => {
                        let value = reader.expect_text()?;
                        wrap_mode_t = parse_wrap_mode(value)?;
                    }
                    "wrapModeR" => {
                        let value = reader.expect_text()?;
                        wrap_mode_r = parse_wrap_mode(value)?;
                    }
                    _ => {
                        // unrecognized, skip
                        reader.skip();
                    }
                }
            });
        }

        todo!()
    }

    fn write(&self, value: &Value, writer: &mut XmlWriter) {
        todo!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Error
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Error)]
pub enum ReadError {
    #[error("parse error")]
    ParseError(#[from] xml::Error),
    #[error("unexpected element")]
    UnexpectedElement,
    #[error("missing attribute")]
    MissingAttribute,
    #[error("non UTF-8 name")]
    NonUtf8Name,
    #[error("integer parse error")]
    ParseIntError(#[from] ParseIntError),
    #[error("integer parse error")]
    ParseFloatError(#[from] ParseFloatError),
    #[error("invalid value format")]
    InvalidValueFormat,
}

impl From<xml::Error> for ReadError {
    fn from(err: xml::Error) -> Self {
        ReadError::ParseError(err)
    }
}

/*
impl From<ParseIntError> for ReadError {
    fn from(err: ParseIntError) -> Self {
        ReadError::ParseIntError(err)
    }
}

impl From<ParseFloatError> for ReadError {
    fn from(err: ParseFloatError) -> Self {
        ReadError::ParseFloatError(err)
    }
}*/

pub type XmlReader<'a> = xml::Reader<&'a [u8]>;
pub type XmlWriter<'a> = xml::Writer<&'a mut dyn io::Write>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Text value parsers
////////////////////////////////////////////////////////////////////////////////////////////////////
fn parse_primitive_value(text: &str, ty: PrimitiveType) -> Result<Value, ReadError> {
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
}

fn parse_array<T: FromStr>(text: &str, expected: Option<usize>) -> Result<Vec<T>, ReadError> {
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

/*fn parse_vec3(text: &str, expected: Option<usize>) -> Result<glam::Vec3, ReadError> {
    let mut out = glam::Vec3::default();
    for (i,elem) in text.split(',').enumerate() {
        if i < len {
            return Err(ReadError::InvalidValueFormat)
        }
        out[i] = elem.parse::<f32>()?;
    }
    Ok(out)
}*/

macro_rules! impl_parse_vector {
    ($vec_ty:ty, $elem_ty:ty, $len:literal, $parse_fn:ident) => {
        fn $parse_fn(text: &str) -> Result<$vec_ty, ReadError> {
            let mut out = $vec_ty::default();
            for (i, elem) in text.split(',').enumerate() {
                if i < $len {
                    return Err(ReadError::InvalidValueFormat);
                }
                out[i] = elem.parse::<$elem_ty>()?;
            }
            Ok(out)
        }
    };
}

impl_parse_vector!(glam::Vec2, f32, 2, parse_vec2);
impl_parse_vector!(glam::Vec3A, f32, 3, parse_vec3);
impl_parse_vector!(glam::Vec4, f32, 4, parse_vec4);

impl_parse_vector!(glam::IVec2, i32, 2, parse_ivec2);
impl_parse_vector!(glam::IVec3A, i32, 3, parse_ivec3);
impl_parse_vector!(glam::IVec4, i32, 4, parse_ivec4);

impl_parse_vector!(glam::UVec2, u32, 2, parse_uvec2);
impl_parse_vector!(glam::UVec3A, u32, 3, parse_uvec3);
impl_parse_vector!(glam::UVec4, u32, 4, parse_uvec4);

impl_parse_vector!(glam::BVec2, bool, 2, parse_bvec2);
impl_parse_vector!(glam::BVec3A, bool, 3, parse_bvec3);
impl_parse_vector!(glam::BVec4, bool, 4, parse_bvec4);

/// Reads a primitive type from an XML element.
fn read_primitive_value(reader: &mut XmlReader, ty: PrimitiveType) -> Result<Value, ReadError> {
    match reader.read_event_unbuffered()? {
        Event::Text(text) => {
            let str = reader.decode(text.escaped())?;
            parse_primitive_value(str, ty)
        }
        _ => Err(ReadError::UnexpectedElement),
    }
}

/// Reads a vector type from an XML element.
fn read_vector_value(reader: &mut XmlReader, elem_ty: PrimitiveType, len: u8) -> Result<Value, ReadError> {
    let text = match reader.read_event_unbuffered()? {
        Event::Text(text) => reader.decode(text.escaped())?,
        _ => Err(ReadError::UnexpectedElement),
    };
    let value = match (elem_ty, len) {
        (PrimitiveType::Int, 2) => Value::IVec2(parse_ivec2(text)?),
        (PrimitiveType::Int, 3) => Value::IVec3(parse_ivec3(text)?),
        (PrimitiveType::Int, 4) => Value::IVec4(parse_ivec4(text)?),
        (PrimitiveType::Float, 2) => Value::Vec2(parse_vec2(text)?),
        (PrimitiveType::Float, 3) => Value::Vec3(parse_vec3(text)?),
        (PrimitiveType::Float, 4) => Value::Vec4(parse_vec4(text)?),
        (PrimitiveType::UnsignedInt, 2) => Value::UVec2(parse_uvec2(text)?),
        (PrimitiveType::UnsignedInt, 3) => Value::UVec3(parse_uvec3(text)?),
        (PrimitiveType::UnsignedInt, 4) => Value::UVec4(parse_uvec4(text)?),
        (PrimitiveType::Bool, 2) => Value::BVec2(parse_bvec2(text)?),
        (PrimitiveType::Bool, 3) => Value::BVec3(parse_bvec3(text)?),
        (PrimitiveType::Bool, 4) => Value::BVec4(parse_bvec4(text)?),
        _ => unimplemented!(),
    };
    Ok(value)
}

fn read_string(reader: &mut XmlReader) -> Result<String, ReadError> {
    match reader.read_event_unbuffered()? {
        Event::Text(text) => {
            let str = reader.decode(text.escaped())?;
            Ok(str.to_string())
        }
        _ => Err(ReadError::UnexpectedElement),
    }
}

fn read_sampler(reader: &mut XmlReader) -> Result<String, ReadError> {
    match reader.read_event_unbuffered()? {
        Event::Start(text) => match text.name() {
            b"wrapModeS" => {}
            b"wrapModeT" => {}
            b"wrapModeR" => {}
            b"minFilter" => {}
            b"magFilter" => {}
            b"borderColor" => {}
            b"mipLodBias" => {}
            b"mipLodBias" => {}
        },
        Event::Text(text) => {
            let str = reader.decode(text.escaped())?;
            Ok(str.to_string())
        }
        _ => Err(ReadError::UnexpectedElement),
    }
}

/// Reads a value from an XML element.
fn read_value(
    reader: &mut XmlReader,
    type_desc: &TypeDesc,
    element: &xml::events::BytesStart,
) -> Result<Value, ReadError> {
    match *type_desc {
        TypeDesc::Primitive(primitive_type) => read_primitive_value(reader, primitive_type),
        TypeDesc::Vector { elem_ty, len } => read_vector_value(reader, elem_ty, len),
    }
}

fn read_param(
    reader: &mut XmlReader,
    element: &xml::events::BytesStart,
    parent_path: &Path,
) -> Result<Param, ReadError> {
    let mut name = None;
    let mut ty = None;

    let elem_name = reader.decode(element.name())?;

    let ty = match elem_name {
        //------------------------------------
        // Builtin types
        "float" => TypeDesc::FLOAT,
        "int" => TypeDesc::INT,
        "uint" => TypeDesc::UNSIGNED_INT,
        "double" => TypeDesc::DOUBLE,
        "vec2" => TypeDesc::VEC2,
        "vec3" => TypeDesc::VEC3,
        "vec4" => TypeDesc::VEC4,
        "ivec2" => TypeDesc::IVEC2,
        "ivec3" => TypeDesc::IVEC3,
        "ivec4" => TypeDesc::IVEC4,
        "uvec2" => TypeDesc::UVEC2,
        "uvec3" => TypeDesc::UVEC3,
        "uvec4" => TypeDesc::UVEC4,
        "sampler" => TypeDesc::SAMPLER,
        "texture1D" => TypeDesc::SampledImage(Arc::new(typedesc::SampledImageType {
            sampled_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim2D,
            ms: false,
        })),
        "texture2D" => TypeDesc::SampledImage(Arc::new(typedesc::SampledImageType {
            sampled_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim2D,
            ms: false,
        })),
        "texture3D" => TypeDesc::SampledImage(Arc::new(typedesc::SampledImageType {
            sampled_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim3D,
            ms: false,
        })),
        "textureCube" => TypeDesc::SampledImage(Arc::new(typedesc::SampledImageType {
            sampled_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::DimCube,
            ms: false,
        })),
        "image1D" => TypeDesc::Image(Arc::new(typedesc::ImageType {
            element_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim2D,
            ms: false,
        })),
        "image2D" => TypeDesc::Image(Arc::new(typedesc::ImageType {
            element_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim2D,
            ms: false,
        })),
        "image3D" => TypeDesc::Image(Arc::new(typedesc::ImageType {
            element_ty: PrimitiveType::Float,
            dim: typedesc::ImageDimension::Dim3D,
            ms: false,
        })),
        //------------------------------------
        // Custom types
        _ => unimplemented!(),
    };

    todo!()
}

/// Reads a node.
fn read_node(reader: &mut XmlReader, element: &xml::events::BytesStart, parent_path: &Path) -> Result<Node, ReadError> {
    let mut name = None;
    let mut op = None;

    for attribute in element.attributes() {
        let attribute = attribute?;
        match attribute.key {
            b"name" => name = Some(attribute.unescape_and_decode_value(reader)?),
            b"op" => name = Some(attribute.unescape_and_decode_value(reader)?),
        }
    }

    let name = name.ok_or(ReadError::MissingAttribute)?;
    let path = parent_path.join(name.into());
    let mut node = Node::new(0, path.clone());

    loop {
        match reader.read_event_unbuffered()? {
            xml::events::Event::End(e) if e.name() == b"node" => break,

            xml::events::Event::Start(e) => match e.name() {
                _ => {
                    let param = read_param(reader, &e, &path)?;
                    node.attributes.insert(param.name(), param)
                }
            },

            _ => {}
        }
    }

    Ok(node)
}

pub fn load_document(xml: &mut XmlReader) -> Result<Document, ReadError> {
    match xml.read_event_unbuffered()? {
        Event::Start(e) => {
            match e.name() {
                b"document" => {
                    //
                }
            }
        }
        Event::End(_) => {}
        Event::Empty(_) => {}
        Event::Text(_) => {}
        Event::Comment(_) => {}
        Event::CData(_) => {}
        Event::Decl(_) => {}
        Event::PI(_) => {}
        Event::DocType(_) => {}
        Event::Eof => {}
    }
}

pub struct XmlFileReader<B: BufRead> {
    reader: quick_xml::Reader<B>,
}

impl<B: BufRead> XmlFileReader<B> {
    pub fn read_event(&mut self) -> Result<Event, ReadError> {
        self.reader.read_event(&mut self.buf)
    }

    pub fn expect(&mut self, name: &[u8], buf: &mut Vec<u8>) -> Result<&[u8], ReadError> {
        match self.read_event()? {
            Event::Start(ref e) if e.name() == name => Ok(e.name()),
            _ => Err(ReadError::UnexpectedElement),
        }
    }

    pub fn read_node(&mut self, buf: &mut Vec<u8>, parent_path: Path) -> Result<Node, quick_xml::Error> {
        let mut node = Node::new();

        match self.reader.read_event(buf)? {
            Event::Start(ref e) => {
                match e.name()? {
                    b"node" => {
                        //let mut child_node = self.read_node(buf, )
                    }
                }
                //let child_node = self.read_node(buf, )
            }
        }
    }

    pub fn read_document(&mut self, buf: &mut Vec<u8>) -> Result<Document, quick_xml::Error> {
        let mut document = Document::new();

        loop {
            match self.reader.read_event(buf)? {
                Event::Start(ref e) => match e.name() {
                    b"node" => {
                        let node = self.read_node(buf)?;
                        document.root.children.insert(node.name())
                    }
                },
                Event::End(ref e) if e.name() == b"document" => break,
                Event::Empty(_) => {}
                Event::Text(_) => {}
                Event::Comment(_) => {}
                Event::CData(_) => {}
                Event::Decl(_) => {}
                Event::PI(_) => {}
                Event::DocType(_) => {}
                Event::Eof => {}
            }
        }

        Ok(())
    }
}

pub fn load_network<B: BufRead>(reader: &mut quick_xml::Reader<B>) -> Result<Node, anyhow::Error> {
    let mut buf = Vec::new();

    match reader.read_event(&mut buf) {
        Ok(Event::Start(ref e)) => match e.name() {
            b"network" => {
                // generic network node
            }
            b"node" => {
                // generic node
            }
            other => {
                // other node class
            }
        },
        Err(_) => {}
    }

    Ok(())
}

pub fn load_document<B: BufRead>(reader: &mut quick_xml::Reader<B>) -> Result<Document, anyhow::Error> {
    match reader.read_event(&mut buf) {}
}

#[cfg(test)]
mod tests {
    use artifice::model::parser::Token;
    use logos::Logos;

    #[test]
    fn test_lexer() {
        let mut lex: logos::Lexer<Token> = Token::lexer(
            r#"
               load <OpLoad> {
                   image input:filePath = "data/image.jpg";
                   image output:output;
               }
            "#,
        );

        while let Some(token) = lex.next() {
            eprintln!("{:?}", token);
        }
    }
}

// operators:
// - read: read an image from a file
// - scene: loads a scene from a file (GLTF, maybe USD?)
// - display: image to display
// - open-image-pipeline: begins a fragment program, taking one or more textures as input
// - close-image-pipeline: "closes" a fragment stream, collecting all fragments into an output texture

// Ports:
// - a "port" can be defined over the namespaces of parameters, in which case all of the params in the namespace show up in the UI as a single port
// - params can have "autoconnect patterns" that describe how they automatically connect to values on their input port
//   - e.g. `normals` would autoconnect to a value on the bus called "normals"

// XML is so fucking annoying to parse with event-based parsers
