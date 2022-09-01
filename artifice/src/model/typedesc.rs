//! Type description.
use kyute_common::Atom;
use std::{collections::HashMap, fmt, sync::Arc};

/// Primitive value types.
///
/// Scalar values of integral and floating-point types.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    Int,
    UnsignedInt,
    Float,
    Double,
    Bool,
}

impl PrimitiveType {
    pub fn display_glsl(&self) -> impl fmt::Display {
        PrimitiveTypeDisplayGlsl(*self)
    }
}

struct PrimitiveTypeDisplayGlsl(PrimitiveType);

impl fmt::Display for PrimitiveTypeDisplayGlsl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::UnsignedInt => write!(f, "uint"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::Double => write!(f, "double"),
            PrimitiveType::Bool => write!(f, "bool"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArrayType {
    pub elem_ty: TypeDesc,
    pub len: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Field {
    pub ty: TypeDesc,
    pub name: Atom,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StructType {
    pub name: Atom,
    pub fields: Vec<Field>,
}

impl StructType {
    pub fn display_glsl(&self) -> impl fmt::Display + '_ {
        StructTypeDisplayGlsl(self)
    }
}

struct StructTypeDisplayGlsl<'a>(&'a StructType);

impl<'a> fmt::Display for StructTypeDisplayGlsl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "struct {} {{", self.0.name)?;
        for (i, field) in self.0.fields.iter().enumerate() {
            write!(f, "{} {};", field.ty.display_glsl(), field.name)?;
        }
        write!(f, "}};")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ImageDimension {
    Dim1D,
    Dim2D,
    Dim3D,
    DimCube,
    Dim1DArray,
    Dim2DArray,
}

/// Sampled image type
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SampledImageType {
    pub sampled_ty: PrimitiveType,
    pub dim: ImageDimension,
    pub ms: bool,
}

impl SampledImageType {
    pub fn display_glsl(&self) -> impl fmt::Display + '_ {
        SampledImageTypeDisplayGlsl(self)
    }
}

struct SampledImageTypeDisplayGlsl<'a>(&'a SampledImageType);

impl<'a> fmt::Display for SampledImageTypeDisplayGlsl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.sampled_ty {
            PrimitiveType::Int => write!(f, "i")?,
            PrimitiveType::UnsignedInt => write!(f, "u")?,
            PrimitiveType::Double => write!(f, "d")?,
            PrimitiveType::Bool => write!(f, "b")?,
            _ => {}
        }

        write!(f, "texture")?;

        match self.0.dim {
            ImageDimension::Dim1D => write!(f, "1D")?,
            ImageDimension::Dim2D => write!(f, "2D")?,
            ImageDimension::Dim3D => write!(f, "3D")?,
            ImageDimension::DimCube => write!(f, "Cube")?,
            ImageDimension::Dim1DArray => write!(f, "1DArray")?,
            ImageDimension::Dim2DArray => write!(f, "2DArray")?,
        }

        if self.0.ms {
            write!(f, "MS")?
        }

        Ok(())
    }
}

/// Unsampled image type
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ImageType {
    pub element_ty: PrimitiveType,
    pub dim: ImageDimension,
    pub ms: bool,
}

struct ImageTypeDisplayGlsl<'a>(&'a ImageType);

impl<'a> fmt::Display for ImageTypeDisplayGlsl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.element_ty {
            PrimitiveType::Int => write!(f, "i")?,
            PrimitiveType::UnsignedInt => write!(f, "u")?,
            PrimitiveType::Double => write!(f, "d")?,
            PrimitiveType::Bool => write!(f, "b")?,
            _ => {}
        }

        write!(f, "image")?;

        match self.0.dim {
            ImageDimension::Dim1D => write!(f, "1D")?,
            ImageDimension::Dim2D => write!(f, "2D")?,
            ImageDimension::Dim3D => write!(f, "3D")?,
            ImageDimension::DimCube => write!(f, "Cube")?,
            ImageDimension::Dim1DArray => write!(f, "1DArray")?,
            ImageDimension::Dim2DArray => write!(f, "2DArray")?,
        }

        if self.0.ms {
            write!(f, "MS")?
        }

        Ok(())
    }
}

impl ImageType {
    pub fn display_glsl(&self) -> impl fmt::Display + '_ {
        ImageTypeDisplayGlsl(self)
    }
}

/// Describes the data type of a value.
///
/// This enum is modeled after SPIR-V (and GLSL) data types, so it is suited to describe the types
/// of a shader interface. However, it also contains types not directly usable in a shader, such as
/// strings.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeDesc {
    Void,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Vector type (ty,size).
    Vector {
        elem_ty: PrimitiveType,
        len: u8,
    },
    /// Matrix type (ty,rows,cols).
    Matrix {
        elem_ty: PrimitiveType,
        rows: u8,
        columns: u8,
    },
    /// Array type. (typedesc + length + stride)
    Array {
        elem_ty: Arc<TypeDesc>,
        len: u32,
    },
    /// Runtime array type. Array without a known length.
    RuntimeArray(Arc<TypeDesc>),
    /// Structure type (array of (offset, type) tuples).
    Struct(Arc<StructType>),
    /// Sampled image type (e.g. `texture2D`).
    SampledImage(Arc<SampledImageType>),
    /// Unsampled image type (e.g. `image2D`).
    Image(Arc<ImageType>),
    /// Pointer to data.
    Pointer(Arc<TypeDesc>),
    /// Sampler.
    Sampler,
    /// Shadow sampler (`samplerShadow`)
    ShadowSampler,
    /// Strings.
    String,
    Unknown,
}

impl TypeDesc {
    pub const VOID: TypeDesc = TypeDesc::Void;
    pub const BOOL: TypeDesc = TypeDesc::Primitive(PrimitiveType::Bool);
    pub const INT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Int);
    pub const UNSIGNED_INT: TypeDesc = TypeDesc::Primitive(PrimitiveType::UnsignedInt);
    pub const FLOAT: TypeDesc = TypeDesc::Primitive(PrimitiveType::Float);
    pub const DOUBLE: TypeDesc = TypeDesc::Primitive(PrimitiveType::Double);
    pub const SAMPLER: TypeDesc = TypeDesc::Sampler;
    pub const SAMPLER_SHADOW: TypeDesc = TypeDesc::ShadowSampler;
    pub const VEC2: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 2,
    };
    pub const VEC3: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 3,
    };
    pub const VEC4: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Float,
        len: 4,
    };
    pub const DVEC2: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Double,
        len: 2,
    };
    pub const DVEC3: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Double,
        len: 3,
    };
    pub const DVEC4: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Double,
        len: 4,
    };
    pub const IVEC2: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 2,
    };
    pub const IVEC3: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 3,
    };
    pub const IVEC4: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Int,
        len: 4,
    };
    pub const UVEC2: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::UnsignedInt,
        len: 2,
    };
    pub const UVEC3: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::UnsignedInt,
        len: 3,
    };
    pub const UVEC4: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::UnsignedInt,
        len: 4,
    };
    pub const MAT2: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 2,
        columns: 2,
    };
    pub const MAT3: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 3,
        columns: 3,
    };
    pub const MAT4: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 4,
    };
    pub const MAT2X3: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 3,
        columns: 2,
    };
    pub const MAT2X4: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 5,
    };
    pub const MAT3X2: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 2,
        columns: 3,
    };
    pub const MAT3X4: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 4,
        columns: 3,
    };
    pub const MAT4X2: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 2,
        columns: 4,
    };
    pub const MAT4X3: TypeDesc = TypeDesc::Matrix {
        elem_ty: PrimitiveType::Float,
        rows: 3,
        columns: 4,
    };

    /// Returns whether the described type is usable in a shader.
    pub fn is_shader_representable(&self) -> bool {
        match self {
            TypeDesc::String => false,
            _ => true,
        }
    }

    /// Returns whether instances of the described type can't be stored in a buffer.
    pub fn is_opaque(&self) -> bool {
        match self {
            TypeDesc::Void => true,
            TypeDesc::Primitive(_) | TypeDesc::Vector { .. } | TypeDesc::Matrix { .. } => false,
            TypeDesc::Array { elem_ty, .. } => elem_ty.is_opaque(),
            TypeDesc::RuntimeArray(_) => true,
            TypeDesc::Struct(_) => false,
            TypeDesc::SampledImage(_) => true,
            TypeDesc::Image(_) => true,
            TypeDesc::Pointer(_) => true, // ??
            TypeDesc::Sampler => true,
            TypeDesc::ShadowSampler => true,
            TypeDesc::String => true,
            TypeDesc::Unknown => true,
        }
    }
}

struct TypeDescGlslDisplay<'a>(&'a TypeDesc);

impl<'a> fmt::Display for TypeDescGlslDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // most of the impl here was inferred by copilot
        match self.0 {
            TypeDesc::Void => {
                write!(f, "void")
            }
            TypeDesc::Primitive(primitive_ty) => {
                write!(f, "{}", primitive_ty.display_glsl())
            }
            TypeDesc::Vector { elem_ty, len } => match elem_ty {
                PrimitiveType::Int => write!(f, "ivec{}", len),
                PrimitiveType::UnsignedInt => write!(f, "uvec{}", len),
                PrimitiveType::Float => write!(f, "vec{}", len),
                PrimitiveType::Double => write!(f, "dvec{}", len),
                PrimitiveType::Bool => write!(f, "bvec{}", len),
            },
            TypeDesc::Matrix { elem_ty, rows, columns } => {
                match elem_ty {
                    PrimitiveType::Float => write!(f, "mat{}{}", rows, columns),
                    PrimitiveType::Double => write!(f, "dmat{}{}", rows, columns),
                    // those are not valid GLSL, but whatever
                    PrimitiveType::Int => write!(f, "imat{}{}", rows, columns),
                    PrimitiveType::UnsignedInt => write!(f, "umat{}{}", rows, columns),
                    PrimitiveType::Bool => write!(f, "bmat{}{}", rows, columns),
                }
            }
            TypeDesc::Array { elem_ty, len } => {
                write!(f, "{}[{}]", elem_ty.display_glsl(), len)
            }
            TypeDesc::RuntimeArray(ty) => {
                write!(f, "{}[]", ty.display_glsl())
            }
            TypeDesc::Struct(struct_ty) => {
                write!(f, "{}", struct_ty.display_glsl())
            }
            TypeDesc::SampledImage(sampled_image_ty) => {
                write!(f, "{}", sampled_image_ty.display_glsl())
            }
            TypeDesc::Image(image_ty) => {
                write!(f, "{}", image_ty.display_glsl())
            }
            TypeDesc::Pointer(ptr) => {
                // not valid GLSL
                write!(f, "{}*", ptr.display_glsl())
            }
            TypeDesc::Sampler => {
                write!(f, "sampler")
            }
            TypeDesc::ShadowSampler => {
                write!(f, "samplerShadow")
            }
            TypeDesc::String => {
                // not valid GLSL
                write!(f, "string")
            }
            TypeDesc::Unknown => {
                write!(f, "unknown")
            }
        }
    }
}

impl TypeDesc {
    pub fn display_glsl(&self) -> impl fmt::Display + '_ {
        TypeDescGlslDisplay(self)
    }
}
