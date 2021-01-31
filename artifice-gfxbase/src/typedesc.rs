//! Type descriptions of buffer data.
use spirv_headers::{Dim, ImageFormat};

/// Primitive data types.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    /// 32-bit signed integer
    Int,
    /// 32-bit unsigned integer
    UnsignedInt,
    /// 16-bit half float (unused)
    Half,
    /// 32-bit floating-point value
    Float,
    /// 64-bit floating-point value
    Double,
    /// Boolean.
    Bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ImageType<'tcx> {
    pub sampled_ty: &'tcx TypeDesc<'tcx>,
    pub format: ImageFormat,
    pub dimensions: Dim,
}

/// Describes a data type used inside a SPIR-V shader
/// (e.g. the type of a uniform, or the type of vertex attributes as seen by the shader).
///
/// TypeDescs are slightly different from Formats:
/// the latter describes the precise bit layout, packing, numeric format, and interpretation
/// of individual data elements, while the former describes unpacked data as seen inside shaders.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeDesc<'tcx> {
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Array type. (typedesc + length + stride)
    Array {
        elem_ty: &'tcx TypeDesc<'tcx>,
        len: usize,
    },
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
    /// Structure type (array of (offset, type) tuples).
    Struct {
        fields: &'tcx [&'tcx TypeDesc<'tcx>],
    },
    /// Image type.
    Image(ImageType<'tcx>),
    /// Combination of an image and sampling information.
    SampledImage(&'tcx ImageType<'tcx>),
    Void,
    /// Pointer to data.
    Pointer(&'tcx TypeDesc<'tcx>),
    Unknown,
}

impl<'tcx> TypeDesc<'tcx> {
    pub fn element_type(&self) -> Option<&'tcx TypeDesc<'tcx>> {
        match self {
            TypeDesc::Array { elem_ty, .. } => Some(*elem_ty),
            TypeDesc::Pointer(elem_ty) => Some(*elem_ty),
            _ => None,
        }
    }

    pub fn pointee_type(&self) -> Option<&'tcx TypeDesc<'tcx>> {
        match self {
            TypeDesc::Pointer(elem_ty) => Some(*elem_ty),
            _ => None,
        }
    }
}
