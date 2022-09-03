use indexmap::IndexSet;
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Index, IndexMut, Range},
    sync::{atomic, atomic::AtomicUsize, Arc},
};

#[repr(transparent)]
pub struct Id<T>(NonZeroU32, PhantomData<fn() -> T>);

impl<T> Id<T> {
    pub fn index(&self) -> usize {
        (self.0.get() - 1) as usize
    }

    pub fn dummy() -> Id<T> {
        unsafe { Id(NonZeroU32::new_unchecked(u32::MAX), PhantomData) }
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id(self.0, PhantomData)
    }
}

impl<T> Copy for Id<T> {}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct IdRange<T>(Id<T>, Id<T>);

impl<T> IdRange<T> {
    pub fn range(&self) -> Range<usize> {
        self.0.index()..self.1.index()
    }
}

impl<T> Clone for IdRange<T> {
    fn clone(&self) -> Self {
        IdRange(self.0, self.1)
    }
}

impl<T> Copy for IdRange<T> {}

impl<T> fmt::Debug for IdRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.0 .0, self.1 .0)
    }
}

impl<T> PartialEq for IdRange<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 .0, self.1 .0) == (other.0 .0, other.1 .0)
    }
}

impl<T> Eq for IdRange<T> {}

impl<T> PartialOrd for IdRange<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.0 .0, self.1 .0).partial_cmp(&(other.0 .0, other.1 .0))
    }
}

impl<T> Ord for IdRange<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.0 .0, self.1 .0).cmp(&(other.0 .0, other.1 .0))
    }
}

impl<T> Hash for IdRange<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub struct Arena<T> {
    pub items: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena { items: vec![] }
    }

    pub fn push(&mut self, item: T) -> Id<T> {
        self.items.push(item);
        unsafe { Id(NonZeroU32::new_unchecked(self.items.len() as u32), PhantomData) }
    }

    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    fn index(&self, index: Id<T>) -> &Self::Output {
        &self.items[(index.0.get() - 1) as usize]
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, index: Id<T>) -> &mut Self::Output {
        &mut self.items[(index.0.get() - 1) as usize]
    }
}

impl<T> Index<IdRange<T>> for Arena<T> {
    type Output = [T];

    fn index(&self, index: IdRange<T>) -> &Self::Output {
        &self.items[index.range()]
    }
}

#[derive(Debug)]
pub struct UniqueArena<T> {
    set: IndexSet<T>,
}

impl<T: Hash + Eq> UniqueArena<T> {
    pub fn new() -> UniqueArena<T> {
        UniqueArena { set: IndexSet::new() }
    }

    pub fn add(&mut self, item: T) -> Id<T> {
        let index = self.set.insert_full(item).0;
        unsafe { Id(NonZeroU32::new_unchecked((index + 1) as u32), PhantomData) }
    }
}

impl<T> Index<Id<T>> for UniqueArena<T> {
    type Output = T;

    fn index(&self, index: Id<T>) -> &Self::Output {
        &self.set[(index.0.get() - 1) as usize]
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

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
pub struct Field {
    pub ty: Id<TypeDesc>,
    pub name: SmolStr,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StructType {
    pub name: SmolStr,
    pub fields: Vec<Field>,
}

/*impl StructType {
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
}*/

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
        elem_ty: Id<TypeDesc>,
        len: u32,
    },
    /// Runtime array type. Array without a known length.
    RuntimeArray(Id<TypeDesc>),
    /// Structure type (array of (offset, type) tuples).
    Struct(StructType),
    /// Sampled image type (e.g. `texture2D`).
    SampledImage(SampledImageType),
    /// Unsampled image type (e.g. `image2D`).
    Image(ImageType),
    /// Pointer to data.
    Pointer(Id<TypeDesc>),
    /// Sampler.
    Sampler,
    /// Shadow sampler (`samplerShadow`)
    ShadowSampler,
    /// Strings.
    String,
    /// Function
    Function {
        return_type: Id<TypeDesc>,
        arguments: Vec<Id<TypeDesc>>,
    },
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
    pub const BVEC2: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Bool,
        len: 2,
    };
    pub const BVEC3: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Bool,
        len: 3,
    };
    pub const BVEC4: TypeDesc = TypeDesc::Vector {
        elem_ty: PrimitiveType::Bool,
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

    pub fn function_return_type(&self) -> Id<TypeDesc> {
        match *self {
            TypeDesc::Function { return_type, .. } => return_type,
            _ => panic!("not a function type"),
        }
    }

    /*/// Returns whether instances of the described type can't be stored in a buffer.
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
    }*/
}

/*struct TypeDescGlslDisplay<'a>(&'a TypeDesc);

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
}*/

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Function {
    //pub return_type: Id<TypeDesc>,
    pub function_type: Id<TypeDesc>,
    pub exprs: Arena<Expr>,
    pub types: Vec<Option<Id<TypeDesc>>>,
}

#[derive(Clone, Debug)]
pub struct TypedExpr {
    ty: TypeDesc,
    expr: Expr,
}

/*/// A place expression (aka LValue).
#[derive(Copy, Clone, Debug)]
pub enum Place {
    ArrayIndex {
        array: Id<Place>,
        index: Id<Expr>,
    },
    Field {
        left: Id<Place>,
        field: u32,
    },
    Error,
}*/

#[derive(Clone, Debug)]
pub enum Expr {
    Argument {
        output: bool,
        name: SmolStr,
        ty: Id<TypeDesc>,
    },
    AccessField {
        place: Id<Expr>,
        index: u32,
    },
    AccessIndex {
        place: Id<Expr>,
        index: Id<Expr>,
    },
    Load {
        pointer: Id<Expr>,
    },
    LocalVariable {
        name: Option<SmolStr>,
        ty: Id<TypeDesc>,
        init: Option<Id<Expr>>,
    },
    Store {
        place: Id<Expr>,
        expr: Id<Expr>,
    },
    Apply {
        func: Id<Function>,
        args: Vec<(usize, Expr)>,
    },
    Minus {
        expr: Id<Expr>,
    },
    Not {
        expr: Id<Expr>,
    },
    FAdd {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    FSub {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    FMul {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    FDiv {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    IAdd {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    ISub {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    IMul {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    IDiv {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Mod {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Shl {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Shr {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Or {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    And {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    BitOr {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    BitAnd {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    BitXor {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Eq {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Ne {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Lt {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Le {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Gt {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    Ge {
        left: Id<Expr>,
        right: Id<Expr>,
    },
    ArrayIndex {
        array: Id<Expr>,
        index: Id<Expr>,
    },
    CompositeConstruct {
        ty: Id<TypeDesc>,
        components: SmallVec<[Id<Expr>; 4]>,
    },
    I32Const(i32),
    U32Const(u32),
    BoolConst(bool),
    F32Const(f32),
    F64Const(f64),
    Error,
    Loop {
        body: Id<Expr>,
        merge: Id<Expr>,
    },
    Selection {
        condition: Id<Expr>,
        true_branch: Id<Expr>,
        false_branch: Option<Id<Expr>>,
        merge: Id<Expr>,
    },
    Merge(Id<Expr>),
    Continue(Id<Expr>),
    Label,
    Branch,
    Noop,
    Return(Option<Id<Expr>>),
    Discard,
    EndFunction,
}

#[derive(Debug)]
pub struct Module {
    pub types: UniqueArena<TypeDesc>,
    pub user_types: HashMap<SmolStr, Id<TypeDesc>>,
    pub functions: Arena<Function>,
    pub error_type: Id<TypeDesc>,

    pub never_type: Id<TypeDesc>,
    pub void_type: Id<TypeDesc>,
    pub bool_type: Id<TypeDesc>,
    pub i32_type: Id<TypeDesc>,
    pub u32_type: Id<TypeDesc>,
    pub f32_type: Id<TypeDesc>,
    pub f64_type: Id<TypeDesc>,

    pub f32x2_type: Id<TypeDesc>,
    pub f32x3_type: Id<TypeDesc>,
    pub f32x4_type: Id<TypeDesc>,

    pub i32x2_type: Id<TypeDesc>,
    pub i32x3_type: Id<TypeDesc>,
    pub i32x4_type: Id<TypeDesc>,

    pub u32x2_type: Id<TypeDesc>,
    pub u32x3_type: Id<TypeDesc>,
    pub u32x4_type: Id<TypeDesc>,

    pub bool2_type: Id<TypeDesc>,
    pub bool3_type: Id<TypeDesc>,
    pub bool4_type: Id<TypeDesc>,

    pub sampler_type: Id<TypeDesc>,
    pub shadow_sampler_type: Id<TypeDesc>,
}

impl Module {
    pub fn new() -> Module {
        let mut types = UniqueArena::new();

        let error_type = types.add(TypeDesc::Unknown);
        let never_type = types.add(TypeDesc::Unknown);
        let void_type = types.add(TypeDesc::Void);
        let bool_type = types.add(TypeDesc::Primitive(PrimitiveType::Bool));
        let i32_type = types.add(TypeDesc::Primitive(PrimitiveType::Int));
        let u32_type = types.add(TypeDesc::Primitive(PrimitiveType::UnsignedInt));
        let f32_type = types.add(TypeDesc::Primitive(PrimitiveType::Float));
        let f64_type = types.add(TypeDesc::Primitive(PrimitiveType::Double));
        let f32x2_type = types.add(TypeDesc::VEC2);
        let f32x3_type = types.add(TypeDesc::VEC3);
        let f32x4_type = types.add(TypeDesc::VEC4);
        let i32x2_type = types.add(TypeDesc::IVEC2);
        let i32x3_type = types.add(TypeDesc::IVEC3);
        let i32x4_type = types.add(TypeDesc::IVEC4);
        let u32x2_type = types.add(TypeDesc::UVEC2);
        let u32x3_type = types.add(TypeDesc::UVEC3);
        let u32x4_type = types.add(TypeDesc::UVEC4);
        let bool2_type = types.add(TypeDesc::BVEC2);
        let bool3_type = types.add(TypeDesc::BVEC3);
        let bool4_type = types.add(TypeDesc::BVEC4);
        let sampler_type = types.add(TypeDesc::Sampler);
        let shadow_sampler_type = types.add(TypeDesc::ShadowSampler);

        Module {
            types,
            user_types: Default::default(),
            never_type,
            functions: Arena::new(),
            error_type,
            void_type,
            bool_type,
            i32_type,
            u32_type,
            f32_type,
            f64_type,
            f32x2_type,
            f32x3_type,
            f32x4_type,
            i32x2_type,
            i32x3_type,
            i32x4_type,
            u32x2_type,
            u32x3_type,
            u32x4_type,
            bool2_type,
            bool3_type,
            bool4_type,
            sampler_type,
            shadow_sampler_type,
        }
    }

    pub fn is_pointer_type(&self, ty: Id<TypeDesc>) -> Option<Id<TypeDesc>> {
        match self.types[ty] {
            TypeDesc::Pointer(elem_ty) => Some(elem_ty),
            _ => None,
        }
    }

    pub fn is_float_scalar_or_vector(&self, ty: Id<TypeDesc>) -> bool {
        ty == self.f32_type || ty == self.f32x2_type || ty == self.f32x3_type || ty == self.f32x4_type
    }

    pub fn is_integer_scalar_or_vector(&self, ty: Id<TypeDesc>) -> bool {
        ty == self.i32_type || ty == self.i32x2_type || ty == self.i32x3_type || ty == self.i32x4_type
    }

    pub fn build_function(&mut self, name: impl Into<SmolStr>) -> FunctionBuilder {
        let return_type = self.void_type;
        FunctionBuilder {
            module: self,
            exprs: Arena::new(),
            types: vec![],
            control_flow_stack: vec![],
            return_type,
        }
    }

    pub fn array_type(&mut self, elem_ty: Id<TypeDesc>, len: u32) -> Id<TypeDesc> {
        self.types.add(TypeDesc::Array { elem_ty, len })
    }

    pub fn runtime_array_type(&mut self, elem_ty: Id<TypeDesc>) -> Id<TypeDesc> {
        self.types.add(TypeDesc::RuntimeArray(elem_ty))
    }

    pub fn user_type(&mut self, name: &str) -> Id<TypeDesc> {
        self.user_types.get(name).cloned().unwrap_or(self.error_type)
    }

    pub fn sampled_image_type(
        &mut self,
        sampled_ty: PrimitiveType,
        dim: ImageDimension,
        multisample: bool,
    ) -> Id<TypeDesc> {
        self.types.add(TypeDesc::SampledImage(SampledImageType {
            sampled_ty,
            dim,
            ms: multisample,
        }))
    }

    pub fn image_type(&mut self, element_ty: PrimitiveType, dim: ImageDimension, ms: bool) -> Id<TypeDesc> {
        self.types.add(TypeDesc::Image(ImageType { element_ty, dim, ms }))
    }
}

/// A control flow block.
pub enum ControlFlowBlock {
    /// Expressions, no branching.
    Block(Block),
    Selection {
        cond: Id<Expr>,
        if_true: Block,
        if_false: Block,
    },
}

// write blocks linearly
// ->

#[derive(Clone, Debug)]
pub struct Block {
    range: IdRange<Expr>,
}

enum CfgScope {
    Loop(Id<Expr>),
}

// loop {
//      if (condition) { break }
// }

// %loop   = Loop %loop-end
// %1      = condition
// %sel    = selection %1 %true %false %sel-end
// %true:
//      merge %loop
// %false:
//      merge %sel
// <loop body>
// branch

// 1           : Loop (2, M+1)
// 2           : Cmp (A,B)
// 3           : Selection (4,N)
// 4-(N-1)     : <true-branch>
// N           : Merge              // merge control flow to merge block of parent construct
// N+1,(M-1)   : <false-branch>
//             : Merge 1            // break loop
//             : <merge instructions>

enum ControlFlow {
    Loop {
        id: Id<Expr>,
        continue_block: Id<Expr>,
        //merge_block: Id<Expr>,
    },
    Selection {
        id: Id<Expr>,
        condition: Id<Expr>,
        true_branch: Id<Expr>,
        false_branch: Option<Id<Expr>>,
        //merge_block: Option<Id<Expr>>,
    },
}

pub struct FunctionBuilder<'module> {
    return_type: Id<TypeDesc>,
    pub module: &'module mut Module,
    pub exprs: Arena<Expr>,
    pub types: Vec<Option<Id<TypeDesc>>>,
    control_flow_stack: Vec<ControlFlow>,
}

// are expressions tied to blocks?
// * no, an expression may be accessed in a child block

// * list of expressions
// Block -> expr range

// BlockBuilder
// * emit_xxx
// *

impl<'module> FunctionBuilder<'module> {
    pub fn finish(mut self) -> Id<Function> {
        self.emit(Expr::EndFunction);
        let module = self.module;
        let arg_count = self
            .exprs
            .items
            .iter()
            .position(|expr| !matches!(expr, Expr::Argument { .. }))
            .unwrap();

        // build function type
        let function_type = TypeDesc::Function {
            return_type: self.return_type,
            arguments: self.exprs.items[0..arg_count]
                .iter()
                .map(|arg| match *arg {
                    Expr::Argument { ty, .. } => ty,
                    _ => unreachable!(),
                })
                .collect(),
        };

        let function_type = module.types.add(function_type);

        module.functions.push(Function {
            function_type,
            exprs: self.exprs,
            types: self.types,
        })
    }

    pub fn argument(&mut self, name: impl Into<SmolStr>, ty: Id<TypeDesc>) -> Id<Expr> {
        if let Some(expr) = self.exprs.last() {
            if !matches!(expr, Expr::Argument { .. }) {
                panic!("cannot add arguments in the current state");
            }
        }

        self.exprs.push(Expr::Argument {
            output: false,
            name: name.into(),
            ty,
        })
    }

    pub fn output_parameter(&mut self, name: impl Into<SmolStr>, ty: Id<TypeDesc>) -> Id<Expr> {
        if let Some(expr) = self.exprs.last() {
            if !matches!(expr, Expr::Argument { .. }) {
                panic!("cannot add arguments in the current state");
            }
        }

        self.exprs.push(Expr::Argument {
            output: true,
            name: name.into(),
            // FIXME should be a pointer type
            ty,
        })
    }

    pub fn emit(&mut self, expr: Expr) -> Id<Expr> {
        self.exprs.push(expr)
    }

    pub fn const_one(&mut self, ty: Id<TypeDesc>) -> Id<Expr> {
        match self.module.types[ty] {
            TypeDesc::Primitive(PrimitiveType::Int) => self.emit(Expr::I32Const(1)),
            TypeDesc::Primitive(PrimitiveType::UnsignedInt) => self.emit(Expr::U32Const(1)),
            TypeDesc::Primitive(PrimitiveType::Float) => self.emit(Expr::F32Const(1.0)),
            TypeDesc::Primitive(PrimitiveType::Double) => self.emit(Expr::F64Const(1.0)),
            _ => self.emit(Expr::Error),
        }
    }

    pub fn error(&mut self) -> Id<Expr> {
        self.exprs.push(Expr::Error)
    }

    pub fn fadd(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::FAdd { left, right })
    }

    pub fn fsub(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::FSub { left, right })
    }

    pub fn fmul(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::FMul { left, right })
    }

    pub fn fdiv(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::FDiv { left, right })
    }

    pub fn iadd(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::IAdd { left, right })
    }

    pub fn isub(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::ISub { left, right })
    }

    pub fn imul(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::IMul { left, right })
    }

    pub fn idiv(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::IDiv { left, right })
    }

    pub fn mod_(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Mod { left, right })
    }

    pub fn eq(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Eq { left, right })
    }

    pub fn ne(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Ne { left, right })
    }

    pub fn gt(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Gt { left, right })
    }

    pub fn ge(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Ge { left, right })
    }

    pub fn lt(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Lt { left, right })
    }

    pub fn le(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Le { left, right })
    }

    pub fn or(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Or { left, right })
    }

    pub fn and(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::And { left, right })
    }

    pub fn bit_or(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::BitOr { left, right })
    }

    pub fn bit_and(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::BitAnd { left, right })
    }

    pub fn bit_xor(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::BitXor { left, right })
    }

    pub fn shl(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Shl { left, right })
    }

    pub fn shr(&mut self, left: Id<Expr>, right: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Shr { left, right })
    }

    pub fn not(&mut self, expr: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::Not { expr })
    }

    pub fn return_(&mut self, expr: Option<Id<Expr>>) -> Id<Expr> {
        self.exprs.push(Expr::Return(expr))
    }

    pub fn discard(&mut self) -> Id<Expr> {
        self.exprs.push(Expr::Discard)
    }

    /*pub fn f_increment(&mut self, place: Id<Expr>) -> Id<Expr> {
        let loaded = self.load(place);
        let ty = self.resolve_type(loaded);
        let one = self.const_one(ty);
        let v = self.add(loaded, one);
        self.store(place, v);
        v
    }

    pub fn f_decrement(&mut self, place: Id<Expr>) -> Id<Expr> {
        let loaded = self.load(place);
        let ty = self.resolve_type(loaded);
        let one = self.const_one(ty);
        let v = self.sub(loaded, one);
        self.store(place, v);
        v
    }

    pub fn f_post_increment(&mut self, place: Id<Expr>) -> Id<Expr> {
        let loaded = self.load(place);
        let ty = self.resolve_type(loaded);
        let one = self.const_one(ty);
        let v = self.fadd(loaded, one);
        self.store(place, v);
        loaded
    }

    pub fn f_post_decrement(&mut self, place: Id<Expr>) -> Id<Expr> {
        let loaded = self.load(place);
        let ty = self.resolve_type(loaded);
        let one = self.const_one(ty);
        let v = self.sub(loaded, one);
        self.store(place, v);
        loaded
    }*/

    //
    pub fn assign(&mut self, place: Id<Expr>, expr: Id<Expr>) -> Id<Expr> {
        self.store(place, expr)
    }

    /*pub fn fadd_assign(&mut self, place: Id<Expr>, expr: Id<Expr>) -> Id<Expr> {
        let a = self.load(place);
        let b = self.add(a, expr);
        self.store(place, b)
    }

    pub fn fsub_assign(&mut self, place: Id<Expr>, expr: Id<Expr>) -> Id<Expr> {
        let a = self.load(place);
        let b = self.sub(a, expr);
        self.store(place, b)
    }

    pub fn fmul_assign(&mut self, place: Id<Expr>, expr: Id<Expr>) -> Id<Expr> {
        let a = self.load(place);
        let b = self.mul(a, expr);
        self.store(place, b)
    }

    pub fn fdiv_assign(&mut self, place: Id<Expr>, expr: Id<Expr>) -> Id<Expr> {
        let a = self.load(place);
        let b = self.div(a, expr);
        self.store(place, b)
    }*/

    pub fn local_variable(&mut self, name: impl Into<SmolStr>, ty: Id<TypeDesc>, init: Option<Id<Expr>>) -> Id<Expr> {
        self.exprs.push(Expr::LocalVariable {
            name: Some(name.into()),
            ty,
            init,
        })
    }

    pub fn array_index(&mut self, array: Id<Expr>, index: Id<Expr>) -> Id<Expr> {
        self.exprs.push(Expr::ArrayIndex { index, array })
    }

    pub fn construct(&mut self, ty: Id<TypeDesc>, components: &[Id<Expr>]) -> Id<Expr> {
        self.exprs.push(Expr::CompositeConstruct {
            ty,
            components: components.into(),
        })
    }

    pub fn vec2(&mut self, x: Id<Expr>, y: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.f32x2_type, &[x, y])
    }

    pub fn vec3(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.f32x3_type, &[x, y, z])
    }

    pub fn vec4(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>, w: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.f32x4_type, &[x, y, z, w])
    }

    pub fn ivec2(&mut self, x: Id<Expr>, y: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.i32x2_type, &[x, y])
    }

    pub fn ivec3(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.i32x3_type, &[x, y, z])
    }

    pub fn ivec4(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>, w: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.i32x4_type, &[x, y, z, w])
    }

    pub fn uvec2(&mut self, x: Id<Expr>, y: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.u32x2_type, &[x, y])
    }

    pub fn uvec3(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.u32x3_type, &[x, y, z])
    }

    pub fn uvec4(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>, w: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.u32x4_type, &[x, y, z, w])
    }

    pub fn bvec2(&mut self, x: Id<Expr>, y: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.bool2_type, &[x, y])
    }

    pub fn bvec3(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.bool3_type, &[x, y, z])
    }

    pub fn bvec4(&mut self, x: Id<Expr>, y: Id<Expr>, z: Id<Expr>, w: Id<Expr>) -> Id<Expr> {
        self.construct(self.module.bool4_type, &[x, y, z, w])
    }

    pub fn loop_(&mut self) -> Id<Expr> {
        // stream sees this, emits a while(true)
        let id = self.exprs.push(Expr::Noop);
        let continue_block = self.exprs.push(Expr::Label);
        self.control_flow_stack.push(ControlFlow::Loop { id, continue_block });
        id
    }

    pub fn continue_(&mut self) -> Id<Expr> {
        for control_flow in self.control_flow_stack.iter().rev() {
            match *control_flow {
                ControlFlow::Loop { id, .. } => {
                    return self.exprs.push(Expr::Continue(id));
                }
                _ => {}
            }
        }
        panic!("no loop expression in current context");
    }

    pub fn break_(&mut self) -> Id<Expr> {
        for control_flow in self.control_flow_stack.iter().rev() {
            match *control_flow {
                ControlFlow::Loop { id, .. } => {
                    return self.exprs.push(Expr::Merge(id));
                }
                _ => {}
            }
        }
        panic!("no loop expression in current context");
    }

    pub fn end_loop(&mut self) {
        let control_flow = self.control_flow_stack.pop().unwrap();
        match control_flow {
            ControlFlow::Loop { id, continue_block } => {
                self.exprs.push(Expr::Continue(id));
                let merge_label = self.exprs.push(Expr::Label);
                self.exprs[id] = Expr::Loop {
                    merge: merge_label,
                    body: continue_block,
                };
            }
            ControlFlow::Selection { .. } => {
                panic!("unexpected control flow operation")
            }
        }
    }

    pub fn if_(&mut self, condition: Id<Expr>) -> Id<Expr> {
        let id = self.exprs.push(Expr::Noop);
        let true_branch = self.exprs.push(Expr::Label);
        self.control_flow_stack.push(ControlFlow::Selection {
            id,
            true_branch,
            false_branch: None,
            condition,
        });
        id
    }

    pub fn else_(&mut self) {
        match *self.control_flow_stack.last_mut().unwrap() {
            ControlFlow::Selection {
                id,
                ref mut false_branch,
                ..
            } => {
                self.exprs.push(Expr::Merge(id));
                let false_label = self.exprs.push(Expr::Label);
                *false_branch = Some(false_label);
            }
            _ => {
                panic!("unexpected control flow operation")
            }
        }
    }

    pub fn end_if(&mut self) {
        let control_flow = self.control_flow_stack.pop().unwrap();
        match control_flow {
            ControlFlow::Selection {
                id,
                condition,
                true_branch,
                false_branch,
            } => {
                self.exprs.push(Expr::Merge(id));
                let merge_label = self.exprs.push(Expr::Label);
                self.exprs[id] = Expr::Selection {
                    condition,
                    true_branch,
                    false_branch,
                    merge: merge_label,
                };
            }
            _ => {
                panic!("unexpected control flow operation")
            }
        }
    }

    pub fn load(&mut self, place: Id<Expr>) -> Id<Expr> {
        self.emit(Expr::Load { pointer: place })
    }

    pub fn store(&mut self, place: Id<Expr>, value: Id<Expr>) -> Id<Expr> {
        self.emit(Expr::Store { place, expr: value })
    }

    pub fn i32_const(&mut self, value: i32) -> Id<Expr> {
        self.emit(Expr::I32Const(value))
    }

    pub fn u32_const(&mut self, value: u32) -> Id<Expr> {
        self.emit(Expr::U32Const(value))
    }

    pub fn f32_const(&mut self, value: f32) -> Id<Expr> {
        self.emit(Expr::F32Const(value))
    }

    pub fn f64_const(&mut self, value: f64) -> Id<Expr> {
        self.emit(Expr::F64Const(value))
    }

    pub fn bool_const(&mut self, value: bool) -> Id<Expr> {
        self.emit(Expr::BoolConst(value))
    }

    pub fn set_type(&mut self, expr: Id<Expr>, ty: Id<TypeDesc>) {
        self.types[expr.index()] = Some(ty);
    }

    pub fn resolve_type(&mut self, expr: Id<Expr>) -> Id<TypeDesc> {
        if self.exprs.items.len() > self.types.len() {
            self.types.resize(self.exprs.items.len(), None);
        }
        if let Some(ty) = self.types[expr.index()] {
            ty
        } else {
            let result = match self.exprs[expr] {
                Expr::AccessField { place, index } => {
                    let ty_place = self.resolve_type(place);
                    match self.module.types[ty_place] {
                        TypeDesc::Struct(ref struct_type) => struct_type.fields[index as usize].ty,
                        _ => self.module.error_type,
                    }
                }
                Expr::AccessIndex { place, index } => {
                    let ty_place = self.resolve_type(place);
                    match self.module.types[ty_place] {
                        TypeDesc::Array { elem_ty, .. } => elem_ty,
                        TypeDesc::RuntimeArray(ty) => ty,
                        _ => self.module.error_type,
                    }
                }
                Expr::Load { pointer } => {
                    let ty_pointer = self.resolve_type(pointer);
                    match self.module.types[ty_pointer] {
                        TypeDesc::Pointer(ty) => ty,
                        _ => self.module.error_type,
                    }
                }
                Expr::LocalVariable { ty, .. } => ty,
                Expr::Store { .. } => self.module.void_type,
                Expr::Apply { func, .. } => {
                    self.module.types[self.module.functions[func].function_type].function_return_type()
                }
                Expr::Minus { expr } => self.resolve_type(expr),
                Expr::Not { expr } => self.resolve_type(expr),
                // TODO
                Expr::FAdd { left, right } => self.resolve_type(left),
                Expr::FSub { left, right } => self.resolve_type(left),
                Expr::FMul { left, right } => self.resolve_type(left),
                Expr::FDiv { left, right } => self.resolve_type(left),
                Expr::IAdd { left, right } => self.resolve_type(left),
                Expr::ISub { left, right } => self.resolve_type(left),
                Expr::IMul { left, right } => self.resolve_type(left),
                Expr::IDiv { left, right } => self.resolve_type(left),
                Expr::ArrayIndex { array, .. } => {
                    let ty_array = self.resolve_type(array);
                    match self.module.types[ty_array] {
                        TypeDesc::Array { elem_ty, .. } => elem_ty,
                        TypeDesc::RuntimeArray(ty) => ty,
                        _ => self.module.error_type,
                    }
                }
                Expr::I32Const(_) => self.module.i32_type,
                Expr::U32Const(_) => self.module.u32_type,
                Expr::BoolConst(_) => self.module.bool_type,
                Expr::F32Const(_) => self.module.f32_type,
                Expr::F64Const(_) => self.module.f64_type,
                Expr::Error => self.module.error_type,
                Expr::Argument { ty, .. } => ty,
                Expr::Loop { .. } => self.module.error_type,
                Expr::Selection { .. } => self.module.error_type,
                Expr::Merge(_) => self.module.never_type,
                Expr::Continue(_) => self.module.never_type,
                Expr::Label => self.module.error_type,
                Expr::Branch => self.module.never_type,
                Expr::Noop => self.module.error_type,
                Expr::Return(_) => self.module.never_type,
                Expr::Discard => self.module.never_type,
                Expr::Mod { .. } => {
                    todo!()
                }
                Expr::Shl { .. } => {
                    todo!()
                }
                Expr::Shr { .. } => {
                    todo!()
                }
                Expr::Eq { .. } => {
                    todo!()
                }
                Expr::Ne { .. } => {
                    todo!()
                }
                Expr::Lt { .. } => {
                    todo!()
                }
                Expr::Le { .. } => {
                    todo!()
                }
                Expr::Gt { .. } => {
                    todo!()
                }
                Expr::Ge { .. } => {
                    todo!()
                }
                Expr::Or { .. } => {
                    todo!()
                }
                Expr::And { .. } => {
                    todo!()
                }
                Expr::BitOr { .. } => {
                    todo!()
                }
                Expr::BitAnd { .. } => {
                    todo!()
                }
                Expr::BitXor { .. } => {
                    todo!()
                }
                Expr::CompositeConstruct { .. } => {
                    todo!()
                }
                Expr::EndFunction => self.module.error_type,
            };
            self.types[expr.index()] = Some(result);
            result
        }
    }
}
