//! Utilities to compute the std140 GLSL layout of types.
use crate::model::{PrimitiveType, TypeDesc};
use thiserror::Error;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Layout
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("encountered and opaque or unrepresentable type")]
    OpaqueType,
}

fn round_up(value: u32, multiple: u32) -> u32 {
    if multiple == 0 {
        return value;
    }
    let remainder = value % multiple;
    if remainder == 0 {
        return value;
    }
    value + multiple - remainder
}

/// Contains information about the layout of a type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Layout {
    /// Alignment
    pub align: u32,
    /// Byte size
    pub size: u32,
    /// Layout of the contents of the type, for array or structs
    pub inner: Option<Box<InnerLayout>>,
}

impl Layout {
    /// Creates a new layout for a scalar element with the specified size and alignment.
    pub const fn with_size_align(size: u32, align: u32) -> Layout {
        Layout {
            align,
            size,
            inner: None,
        }
    }
}

/// Layout of the fields of a struct type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StructLayout {
    /// Offsets of each field.
    pub offsets: Vec<u32>,
    /// Individual layout information of each field.
    pub layouts: Vec<Layout>,
}

impl StructLayout {
    pub fn std140<'a>(fields: impl Iterator<Item = &'a TypeDesc>) -> Result<StructLayout, LayoutError> {
        let (_size, _align, layout) = std140_struct_layout(fields)?;
        Ok(layout)
    }
}

/// Layout of the array elements of a array type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArrayLayout {
    /// Layout of individual array elements.
    pub elem_layout: Layout,
    /// Number of bytes between consecutive array elements.
    pub stride: u32,
}

/// Layout information for arrays or structs.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum InnerLayout {
    Array(ArrayLayout),
    Struct(StructLayout),
}

fn std140_array_layout(elem_ty: &TypeDesc, arraylen: u32) -> Result<(u32, u32, ArrayLayout), LayoutError> {
    let elem_layout = std140_layout(elem_ty)?;
    // alignment = column type align rounded up to vec4 align (16 bytes)
    let base_align = round_up(elem_layout.align, 16);
    let stride = round_up(elem_layout.size, elem_layout.align);
    // total array size = num columns * stride, rounded up to the next multiple of the base alignment.
    // actually the spec says nothing about the 'size' of an element, only about the alignment
    // of the next element in the structure.
    let array_size = round_up(arraylen * stride, base_align);

    Ok((array_size, base_align, ArrayLayout { elem_layout, stride }))
}

fn std140_struct_layout<'a>(fields: impl Iterator<Item = &'a TypeDesc>) -> Result<(u32, u32, StructLayout), Layout> {
    /* If the member is a structure, the base alignment of the structure is N,
    where N is the largest base alignment value of any of its members,
    and rounded up to the base alignment of a vec4.
    The individual members of this sub-structure are then assigned offsets by applying this set of rules recursively,
    where the base offset of the first member of the sub-structure is equal to the aligned offset of the structure.
    The structure may have padding at the end;
    the base offset of the member following the sub-structure is rounded up to the next multiple of the base alignment of the structure.
    */
    // TODO: zero-sized structures?

    let layouts = fields
        .map(|field| std140_layout(field))
        .collect::<Result<Vec<_>, _>>()?;
    let n = layouts.iter().map(|l| l.align).max().unwrap_or(0);
    if n == 0 {
        // skip, no members
        return Ok((
            0,
            0,
            StructLayout {
                offsets: vec![],
                layouts: vec![],
            },
        ));
    }

    // round up to base alignment of vec4
    let n = round_up(n, 16);

    // compute field offsets
    let mut offsets = vec![0; layouts.len()];
    let mut off = 0;
    for i in 0..layouts.len() {
        offsets[i] = off;
        off += layouts[i].size;
    }

    // round up total size to base align
    let size = round_up(off, n);

    Ok((size, n, StructLayout { layouts, offsets }))
}

fn std140_primitive_layout(prim_ty: PrimitiveType) -> Layout {
    match prim_ty {
        PrimitiveType::Int | PrimitiveType::UnsignedInt | PrimitiveType::Float => Layout {
            size: 4,
            align: 4,
            inner: None,
        },
        _ => unimplemented!(),
    }
}

fn std140_vector_layout(prim_ty: PrimitiveType, len: u8) -> Layout {
    let Layout { size: n, .. } = std140_primitive_layout(prim_ty);
    match len {
        2 => Layout {
            align: 2 * n,
            size: 2 * n,
            inner: None,
        },
        3 => Layout {
            align: 4 * n,
            size: 3 * n,
            inner: None,
        },
        4 => Layout {
            align: 4 * n,
            size: 4 * n,
            inner: None,
        },
        _ => panic!("unsupported vector size"),
    }
}

/// Computes the layout of a TypeDesc, using std140 rules.
fn std140_layout(ty: &TypeDesc) -> Result<Layout, LayoutError> {
    match *ty {
        TypeDesc::Primitive(p) => Ok(std140_primitive_layout(p)),
        TypeDesc::Vector { elem_ty, len } => Ok(std140_vector_layout(elem_ty, len)),
        TypeDesc::Matrix { elem_ty, rows, columns } => {
            let (size, align, layout) = std140_array_layout(&TypeDesc::Vector { elem_ty, len: rows }, columns as u32)?;
            Ok(Layout {
                size,
                align,
                inner: Some(Box::new(InnerLayout::Array(layout))),
            })
        }
        TypeDesc::Array { ref elem_ty, len } => match &**elem_ty {
            TypeDesc::Primitive(_) | TypeDesc::Vector { .. } | TypeDesc::Struct { .. } => {
                let (size, align, layout) = std140_array_layout(elem_ty, len);
                Ok(Layout {
                    size,
                    align,
                    inner: Some(Box::new(InnerLayout::Array(layout))),
                })
            }
            ty => panic!("unsupported array element type: {:?}", ty),
        },
        TypeDesc::Struct(ref ty) => {
            let (size, align, layout) = std140_struct_layout(ty.fields.iter().map(|f| &f.ty));
            Layout {
                size,
                align,
                inner: Some(Box::new(InnerLayout::Struct(layout))),
            }
        }
        ref ty => Err(LayoutError::OpaqueType),
    }
}

impl Layout {
    /// Returns the std140 layout of the given type.
    pub fn std140(ty: &TypeDesc) -> Result<Layout, LayoutError> {
        std140_layout(ty)
    }
}
