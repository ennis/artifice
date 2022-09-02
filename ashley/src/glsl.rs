//! GLSL
//! Parsing of shader programs (GLSL snippets).
use crate::{
    ast,
    ast::{Id, TypeDesc},
};
use glsl_lang as glsl;
use glsl_lang::{
    ast::{
        ArraySpecifierData, ArraySpecifierDimensionData, AssignmentOpData, BinaryOpData, CompoundStatement,
        ConditionData, DeclarationData, Expr, ExprData, ExternalDeclarationData, ForInitStatementData,
        FunIdentifierData, FunctionDefinition, FunctionParameterDeclarationData, InitDeclaratorList, Initializer,
        InitializerData, IterationStatementData, JumpStatementData, NodeSpan, SelectionRestStatementData, Statement,
        StatementData, StorageQualifierData, TranslationUnit, TypeQualifierSpecData, TypeSpecifierData,
        TypeSpecifierNonArrayData, UnaryOpData,
    },
    lexer::v2_full::fs::{FileSystem, Lexer, PreprocessorExt},
    parse::{Parse, ParseOptions},
    transpiler::glsl::{
        show_array_spec, show_arrayed_identifier, show_expr, show_function_prototype, show_init_declarator_list,
        show_initializer, show_storage_qualifier, show_translation_unit, show_type_specifier, FormattingState,
    },
    visitor::{HostMut, Visit},
};
use glsl_lang_pp::{
    ext_name,
    processor::{fs::Processor, nodes::ExtensionBehavior, ProcessorState},
};
use smallvec::smallvec;
use smol_str::SmolStr;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashMap,
    fmt,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProgramError
////////////////////////////////////////////////////////////////////////////////////////////////////

/*/// Helper used to associate numeric IDs to source paths.
pub struct FileIdMap {
    /// Maps source IDs to numeric IDs.
    pub paths: Mutex<HashMap<Atom, u32>>,
}

impl FileIdMap {
    /// Creates a new FileIdMap.
    pub fn new() -> Self {
        FileIdMap {
            paths: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the numeric ID for the given source path.
    pub fn id(&self, id: impl Into<Atom>) -> u32 {
        let mut paths = self.paths.lock();
        let count = paths.len() as u32;
        let id = id.into();
        *paths.entry(id).or_insert(count)
    }
}

static mut FILE_ID_MAP: Lazy<FileIdMap> = Lazy::new(|| FileIdMap::new());*/

////////////////////////////////////////////////////////////////////////////////////////////////////
// VFS & preprocessor stuff
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Vfs {
    files: Arc<HashMap<String, String>>,
}

impl Vfs {
    /// Creates a new instance.
    pub fn new() -> Vfs {
        Vfs {
            files: Arc::new(HashMap::new()),
        }
    }

    /// Registers a file.
    pub fn register_source(&mut self, id: impl Into<String>, src: impl Into<String>) {
        Arc::make_mut(&mut self.files)
            .entry(id.into())
            .or_insert_with(|| src.into());
    }

    pub fn dump(&self) {
        eprintln!("Registered source files: ");
        for file in self.files.iter() {
            eprintln!(" - {}", file.0);
        }
        eprintln!();
    }
}

impl FileSystem for Vfs {
    type Error = std::io::Error;

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, Self::Error> {
        Ok(path.to_owned())
    }

    fn exists(&self, path: &Path) -> bool {
        let str = path.to_string_lossy();
        self.files.contains_key(&*str)
    }

    fn read(&self, path: &Path) -> Result<Cow<'_, str>, Self::Error> {
        if let Some(src) = self.files.get(&*path.to_string_lossy()) {
            Ok(Cow::Borrowed(&*src))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        }
    }
}

pub type Preprocessor = Processor<Vfs>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// TypeMap
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Converts a element type into an array type given an ArraySpecifier.
fn apply_array_specifier(
    module: &mut ast::Module,
    elem_ty: Id<ast::TypeDesc>,
    array_spec: &ArraySpecifierData,
) -> Id<ast::TypeDesc> {
    let mut ty = elem_ty;
    for dim in array_spec.dimensions.iter() {
        match dim.content {
            ArraySpecifierDimensionData::Unsized => {
                ty = module.types.add(ast::TypeDesc::RuntimeArray(ty));
            }
            ArraySpecifierDimensionData::ExplicitlySized(ref expr) => match expr.content {
                ExprData::IntConst(value) => {
                    if value < 0 {
                        return module.error_type;

                        /*return Err(ProgramError::interface(
                            expr.span,
                            "array size must be non-negative".to_string(),
                        ));*/
                    }
                    ty = module.array_type(ty, value as u32);
                }
                ExprData::UIntConst(value) => {
                    ty = module.array_type(ty, value as u32);
                }
                _ => return module.error_type,
            },
        }
    }
    ty
}

/// Converts a GLSL type specifier into a `TypeDesc`.
fn type_specifier_to_type_desc(
    module: &mut ast::Module,
    spec: &TypeSpecifierData,
    array_spec: Option<&ArraySpecifierData>,
) -> Id<ast::TypeDesc> {
    let mut ty = match spec.ty.content {
        TypeSpecifierNonArrayData::Void => module.void_type,
        TypeSpecifierNonArrayData::Bool => module.bool_type,
        TypeSpecifierNonArrayData::Int => module.i32_type,
        TypeSpecifierNonArrayData::UInt => module.u32_type,
        TypeSpecifierNonArrayData::Float => module.f32_type,
        TypeSpecifierNonArrayData::Double => module.f64_type,
        TypeSpecifierNonArrayData::Vec2 => module.f32x2_type,
        TypeSpecifierNonArrayData::Vec3 => module.f32x3_type,
        TypeSpecifierNonArrayData::Vec4 => module.f32x4_type,
        TypeSpecifierNonArrayData::DVec2 => todo!(),
        TypeSpecifierNonArrayData::DVec3 => todo!(),
        TypeSpecifierNonArrayData::DVec4 => todo!(),
        TypeSpecifierNonArrayData::BVec2 => module.bool2_type,
        TypeSpecifierNonArrayData::BVec3 => module.bool3_type,
        TypeSpecifierNonArrayData::BVec4 => module.bool4_type,
        TypeSpecifierNonArrayData::IVec2 => module.i32x2_type,
        TypeSpecifierNonArrayData::IVec3 => module.i32x3_type,
        TypeSpecifierNonArrayData::IVec4 => module.i32x4_type,
        TypeSpecifierNonArrayData::UVec2 => module.u32x2_type,
        TypeSpecifierNonArrayData::UVec3 => module.u32x3_type,
        TypeSpecifierNonArrayData::UVec4 => module.u32x4_type,
        TypeSpecifierNonArrayData::Mat2 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 2,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat3 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 3,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat4 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 4,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat22 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 2,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat23 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 2,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat24 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 2,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat32 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 3,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat33 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 3,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat34 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 4,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat42 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 2,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat43 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 3,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::Mat44 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 4,
            elem_ty: ast::PrimitiveType::Float,
        }),
        TypeSpecifierNonArrayData::DMat2 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 2,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat3 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 3,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat4 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 4,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat22 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 2,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat23 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 2,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat24 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 2,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat32 => module.types.add(ast::TypeDesc::Matrix {
            rows: 2,
            columns: 3,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat33 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 3,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat34 => module.types.add(ast::TypeDesc::Matrix {
            rows: 3,
            columns: 4,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat42 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 2,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat43 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 3,
            elem_ty: ast::PrimitiveType::Double,
        }),
        TypeSpecifierNonArrayData::DMat44 => module.types.add(ast::TypeDesc::Matrix {
            rows: 4,
            columns: 4,
            elem_ty: ast::PrimitiveType::Double,
        }),

        TypeSpecifierNonArrayData::Texture1D => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim1D, false)
        }
        TypeSpecifierNonArrayData::Texture2D => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2D, false)
        }
        TypeSpecifierNonArrayData::Texture3D => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim3D, false)
        }
        TypeSpecifierNonArrayData::TextureCube => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::DimCube, false)
        }
        //TypeSpecifierNonArrayData::Texture2DRect => {tyctx.module.sampled_image_type(Float, ast::ImageDimension::Dim2DRect, false)}
        TypeSpecifierNonArrayData::Texture1DArray => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim1DArray, false)
        }
        TypeSpecifierNonArrayData::Texture2DArray => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2DArray, false)
        }
        //TypeSpecifierNonArrayData::TextureBuffer => {}
        TypeSpecifierNonArrayData::Texture2DMs => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2D, true)
        }
        TypeSpecifierNonArrayData::Texture2DMsArray => {
            module.sampled_image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2DArray, true)
        }
        //TypeSpecifierNonArrayData::TextureCubeArray => {tyctx.module.sampled_image_type(Float, ast::ImageDimension::DimCubeArray, false)}
        TypeSpecifierNonArrayData::ITexture1D => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim1D, false)
        }
        TypeSpecifierNonArrayData::ITexture2D => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2D, false)
        }
        TypeSpecifierNonArrayData::ITexture3D => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim3D, false)
        }
        TypeSpecifierNonArrayData::ITextureCube => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::DimCube, false)
        }
        //TypeSpecifierNonArrayData::ITexture2DRect => {}
        TypeSpecifierNonArrayData::ITexture1DArray => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim1DArray, false)
        }
        TypeSpecifierNonArrayData::ITexture2DArray => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2DArray, false)
        }
        //TypeSpecifierNonArrayData::ITextureBuffer => {}
        TypeSpecifierNonArrayData::ITexture2DMs => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2D, true)
        }
        TypeSpecifierNonArrayData::ITexture2DMsArray => {
            module.sampled_image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2DArray, true)
        }
        //TypeSpecifierNonArrayData::ITextureCubeArray => {
        //    tyctx.module.sampled_image_type(Int, ast::ImageDimension::DimCubeArray, false)
        //}
        TypeSpecifierNonArrayData::Image1D => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim1D, false)
        }
        TypeSpecifierNonArrayData::Image2D => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2D, false)
        }
        TypeSpecifierNonArrayData::Image3D => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim3D, false)
        }
        TypeSpecifierNonArrayData::ImageCube => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::DimCube, false)
        }
        TypeSpecifierNonArrayData::Image1DArray => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim1DArray, false)
        }
        TypeSpecifierNonArrayData::Image2DArray => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2DArray, false)
        }
        TypeSpecifierNonArrayData::Image2DMs => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2D, true)
        }
        TypeSpecifierNonArrayData::Image2DMsArray => {
            module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::Dim2DArray, true)
        }
        //TypeSpecifierNonArrayData::ImageCubeArray => {
        //    tyctx.module.image_type(ast::PrimitiveType::Float, ast::ImageDimension::DimCubeArray, false)
        //}
        TypeSpecifierNonArrayData::IImage1D => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim1D, false)
        }
        TypeSpecifierNonArrayData::IImage2D => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2D, false)
        }
        TypeSpecifierNonArrayData::IImage3D => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim3D, false)
        }
        TypeSpecifierNonArrayData::IImageCube => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::DimCube, false)
        }
        TypeSpecifierNonArrayData::IImage1DArray => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim1DArray, false)
        }
        TypeSpecifierNonArrayData::IImage2DArray => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2DArray, false)
        }
        TypeSpecifierNonArrayData::IImage2DMs => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2D, true)
        }
        TypeSpecifierNonArrayData::IImage2DMsArray => {
            module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::Dim2DArray, true)
        }
        //TypeSpecifierNonArrayData::IImageCubeArray => {
        //    tyctx.module.image_type(ast::PrimitiveType::Int, ast::ImageDimension::DimCubeArray, false)
        //}
        TypeSpecifierNonArrayData::AtomicUInt => todo!(),
        TypeSpecifierNonArrayData::UImage1D => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim1D, false)
        }
        TypeSpecifierNonArrayData::UImage2D => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim2D, false)
        }
        TypeSpecifierNonArrayData::UImage3D => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim3D, false)
        }
        //TypeSpecifierNonArrayData::UImageCube => {
        //    tyctx.module.image_type(ast::PrimitiveType::UInt, ast::ImageDimension::DimCube, false)
        //}
        TypeSpecifierNonArrayData::UImage1DArray => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim1DArray, false)
        }
        TypeSpecifierNonArrayData::UImage2DArray => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim2DArray, false)
        }
        //TypeSpecifierNonArrayData::UImageBuffer => {
        //    tyctx.module.image_type(ast::PrimitiveType::UInt, ast::ImageDimension::DimBuffer, false)
        //}
        TypeSpecifierNonArrayData::UImage2DMs => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim2D, true)
        }
        TypeSpecifierNonArrayData::UImage2DMsArray => {
            module.image_type(ast::PrimitiveType::UnsignedInt, ast::ImageDimension::Dim2DArray, true)
        }
        TypeSpecifierNonArrayData::Sampler => module.sampler_type,
        TypeSpecifierNonArrayData::SamplerShadow => module.shadow_sampler_type,
        //TypeSpecifierNonArrayData::UImageCubeArray => {
        //    tyctx.module.image_type(ast::PrimitiveType::UInt, ast::ImageDimension::DimCubeArray, false)
        //}
        TypeSpecifierNonArrayData::Struct(ref s) => {
            let name = s.content.name.as_ref().map(|name| name.content.0.to_string());
            let mut fields = Vec::new();
            for f in s.content.fields.iter() {
                for ident in f.identifiers.iter() {
                    let field_name = ident.ident.to_string();
                    let field_ty = type_specifier_to_type_desc(
                        module,
                        &f.ty.content,
                        ident.content.array_spec.as_ref().map(|spec| &spec.content),
                    );

                    fields.push(ast::Field {
                        name: field_name.into(),
                        ty: field_ty,
                    })
                }
            }

            let span = s.span.unwrap();

            let ty = ast::TypeDesc::Struct(ast::StructType {
                name: name
                    .clone()
                    .unwrap_or(format!(
                        "anon_{}_{:?}_{:?}",
                        span.source_id().number(),
                        span.start().offset,
                        span.end().offset
                    ))
                    .into(),
                fields,
            });
            let id = module.types.add(ty);

            if let Some(name) = name {
                module.user_types.insert(name.into(), id);
            }

            id
        }
        TypeSpecifierNonArrayData::TypeName(ref name) => module.user_type(name.as_str()),
        _ => module.error_type,
    };

    // array specifier attached to the type
    if let Some(ref array_spec) = spec.array_specifier {
        ty = apply_array_specifier(module, ty, &array_spec.content);
    }

    // array specified attached to the identifier
    if let Some(ref array_spec) = array_spec {
        ty = apply_array_specifier(module, ty, array_spec);
    }

    ty
}

/*bitflags! {
    struct StorageQualifiers: u32 {
        const CONST     = 0b0000_0000_0000_0000_0000_0000_0000_0001;
        const INOUT     = 0b0000_0000_0000_0000_0000_0000_0000_0010;
        const IN        = 0b0000_0000_0000_0000_0000_0000_0000_0100;
        const OUT       = 0b0000_0000_0000_0000_0000_0000_0000_1000;
        const CENTROID  = 0b0000_0000_0000_0000_0000_0000_0001_0000;
        const PATCH     = 0b0000_0000_0000_0000_0000_0000_0010_0000;
        const SAMPLE    = 0b0000_0000_0000_0000_0000_0000_0100_0000;
        const UNIFORM   = 0b0000_0000_0000_0000_0000_0000_1000_0000;
        const BUFFER    = 0b0000_0000_0000_0000_0000_0001_0000_0000;
        const SHARED    = 0b0000_0000_0000_0000_0000_0010_0000_0000;
        const COHERENT  = 0b0000_0000_0000_0000_0000_0100_0000_0000;
        const VOLATILE  = 0b0000_0000_0000_0000_0000_1000_0000_0000;
        const RESTRICT  = 0b0000_0000_0000_0000_0001_0000_0000_0000;
        const READONLY  = 0b0000_0000_0000_0000_0010_0000_0000_0000;
        const WRITEONLY = 0b0000_0000_0000_0000_0100_0000_0000_0000;
        const ATTRIBUTE = 0b0000_0000_0000_0000_1000_0000_0000_0000;
        const VARYING   = 0b0000_0000_0000_0001_0000_0000_0000_0000;

        const INTERFACE_MASK = Self::IN.bits | Self::OUT.bits | Self::UNIFORM.bits;
    }
}*/
/*
fn get_storage_qualifiers(ty: &FullySpecifiedTypeData) -> StorageQualifiers {
    let mut qualifiers = StorageQualifiers::empty();
    if let Some(ref ty_qualifier_data) = ty.qualifier {
        for qual in ty_qualifier_data.content.qualifiers.iter() {
            match qual.content {
                TypeQualifierSpecData::Storage(ref data) => match data.content {
                    StorageQualifierData::Const => {
                        qualifiers |= StorageQualifiers::CONST;
                    }
                    StorageQualifierData::InOut => {
                        qualifiers |= StorageQualifiers::INOUT;
                    }
                    StorageQualifierData::In => {
                        qualifiers |= StorageQualifiers::IN;
                    }
                    StorageQualifierData::Out => {
                        qualifiers |= StorageQualifiers::OUT;
                    }
                    StorageQualifierData::Centroid => {
                        qualifiers |= StorageQualifiers::CENTROID;
                    }
                    StorageQualifierData::Patch => {
                        qualifiers |= StorageQualifiers::PATCH;
                    }
                    StorageQualifierData::Sample => {
                        qualifiers |= StorageQualifiers::SAMPLE;
                    }
                    StorageQualifierData::Uniform => {
                        qualifiers |= StorageQualifiers::UNIFORM;
                    }
                    StorageQualifierData::Buffer => {
                        qualifiers |= StorageQualifiers::BUFFER;
                    }
                    StorageQualifierData::Shared => {
                        qualifiers |= StorageQualifiers::SHARED;
                    }
                    StorageQualifierData::Coherent => {
                        qualifiers |= StorageQualifiers::COHERENT;
                    }
                    StorageQualifierData::Volatile => {
                        qualifiers |= StorageQualifiers::VOLATILE;
                    }
                    StorageQualifierData::Restrict => {
                        qualifiers |= StorageQualifiers::RESTRICT;
                    }
                    StorageQualifierData::ReadOnly => {
                        qualifiers |= StorageQualifiers::READONLY;
                    }
                    StorageQualifierData::WriteOnly => {
                        qualifiers |= StorageQualifiers::WRITEONLY;
                    }
                    StorageQualifierData::Attribute => {
                        qualifiers |= StorageQualifiers::ATTRIBUTE;
                    }
                    StorageQualifierData::Varying => {
                        qualifiers |= StorageQualifiers::VARYING;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    qualifiers
}*/

////////////////////////////////////////////////////////////////////////////////////////////////////
//
////////////////////////////////////////////////////////////////////////////////////////////////////

type Scope = HashMap<SmolStr, Id<ast::Expr>>;

/// Converter state for a function
struct FunctionBodyTranslator<'module> {
    builder: ast::FunctionBuilder<'module>,
    errors: &'module mut Vec<String>,
    scopes: Vec<Scope>,
}

impl<'module> FunctionBodyTranslator<'module> {
    /// Makes a variable visible in the current lexical scope.
    fn define_variable(&mut self, name: impl Into<SmolStr>, expr: Id<ast::Expr>) {
        self.scopes.last_mut().unwrap().insert(name.into(), expr);
    }

    fn find_variable_in_scope(&self, name: &str) -> Option<Id<ast::Expr>> {
        self.scopes.last().unwrap().get(name).cloned()
    }

    fn enter_scope(&mut self) {
        let new_scope = self.scopes.last().unwrap().clone();
        self.scopes.push(new_scope);
    }

    fn exit_scope(&mut self) {
        self.scopes.pop().expect("unbalanced scopes");
    }

    fn emit_error(&mut self) -> Id<ast::Expr> {
        self.builder.exprs.add(ast::Expr::Error)
    }

    //fn translate_function_call()

    fn translate_place(&mut self, expr: &Expr) -> Id<ast::Expr> {
        match expr.content {
            ExprData::Variable(ref var) => {
                let var = self.find_variable_in_scope(var.as_str());
                if let Some(var) = var {
                    var
                } else {
                    self.emit_error()
                }
            }
            ExprData::Bracket(ref array, ref index) => {
                let array = self.translate_place(array);
                let index = self.translate_expr(index);
                self.builder.emit(ast::Expr::AccessIndex { place: array, index })
            }
            ExprData::Dot(ref expr, ref ident) => {
                let base = self.translate_expr(&expr);
                let ty = self.builder.resolve_type(base);
                match self.builder.module.types[ty] {
                    ast::TypeDesc::Struct(ref struct_type) => {
                        // field access
                        let field_index = struct_type
                            .fields
                            .iter()
                            .position(|field| field.name.as_str() == ident.as_str());
                        if let Some(field_index) = field_index {
                            self.builder.emit(ast::Expr::AccessField {
                                place: base,
                                index: field_index as u32,
                            })
                        } else {
                            self.emit_error()
                        }
                    }
                    ast::TypeDesc::Vector { .. } => {
                        // TODO: swizzle place
                        todo!("swizzle")
                    }
                    _ => self.emit_error(),
                }
            }
            _ => self.emit_error(),
        }
    }

    // what's easier for GLSL output?
    // * expressions have no side effects
    // * all side effects are in statements

    fn translate_expr(&mut self, expr: &Expr) -> Id<ast::Expr> {
        match expr.content {
            ExprData::Variable(ref var) => {
                let var = self.find_variable_in_scope(var.as_str());
                if let Some(var) = var {
                    self.builder.load(var)
                } else {
                    self.emit_error()
                }
            }
            ExprData::IntConst(v) => self.builder.i32_const(v),
            ExprData::UIntConst(v) => self.builder.u32_const(v),
            ExprData::BoolConst(v) => self.builder.bool_const(v),
            ExprData::FloatConst(v) => self.builder.f32_const(v),
            ExprData::DoubleConst(v) => self.builder.f64_const(v),
            ExprData::Unary(ref op, ref expr) => match op.content {
                UnaryOpData::Inc => {
                    let place = self.translate_place(expr);
                    self.builder.increment(place)
                }
                UnaryOpData::Dec => {
                    let place = self.translate_place(expr);
                    self.builder.decrement(place)
                }
                UnaryOpData::Add => todo!(),
                UnaryOpData::Minus => todo!(),
                UnaryOpData::Not => todo!(),
                UnaryOpData::Complement => todo!(),
            },
            ExprData::Binary(ref op, ref left, ref right) => {
                let left = self.translate_expr(left);
                let right = self.translate_expr(right);
                match op.content {
                    BinaryOpData::Or => self.builder.or(left, right),
                    BinaryOpData::Xor => {
                        todo!()
                    }
                    BinaryOpData::And => self.builder.and(left, right),
                    BinaryOpData::BitOr => self.builder.bit_or(left, right),
                    BinaryOpData::BitXor => self.builder.bit_xor(left, right),
                    BinaryOpData::BitAnd => self.builder.bit_and(left, right),
                    BinaryOpData::Equal => self.builder.eq(left, right),
                    BinaryOpData::NonEqual => self.builder.ne(left, right),
                    BinaryOpData::Lt => self.builder.lt(left, right),
                    BinaryOpData::Gt => self.builder.gt(left, right),
                    BinaryOpData::Lte => self.builder.le(left, right),
                    BinaryOpData::Gte => self.builder.ge(left, right),
                    BinaryOpData::LShift => self.builder.shl(left, right),
                    BinaryOpData::RShift => self.builder.shr(left, right),
                    BinaryOpData::Add => self.builder.add(left, right),
                    BinaryOpData::Sub => self.builder.sub(left, right),
                    BinaryOpData::Mult => self.builder.mul(left, right),
                    BinaryOpData::Div => self.builder.div(left, right),
                    BinaryOpData::Mod => self.builder.mod_(left, right),
                }
            }
            ExprData::Ternary(_, _, _) => {
                todo!()
            }
            ExprData::Assignment(ref place, ref op, ref expr) => {
                let place = self.translate_place(place);
                let expr = self.translate_expr(expr);
                match op.content {
                    AssignmentOpData::Equal => self.builder.assign(place, expr),
                    AssignmentOpData::Mult => self.builder.mul_assign(place, expr),
                    AssignmentOpData::Div => self.builder.div_assign(place, expr),
                    AssignmentOpData::Mod => {
                        todo!()
                    }
                    AssignmentOpData::Add => self.builder.add_assign(place, expr),
                    AssignmentOpData::Sub => self.builder.sub_assign(place, expr),
                    AssignmentOpData::LShift => {
                        todo!()
                    }
                    AssignmentOpData::RShift => {
                        todo!()
                    }
                    AssignmentOpData::And => {
                        todo!()
                    }
                    AssignmentOpData::Xor => {
                        todo!()
                    }
                    AssignmentOpData::Or => {
                        todo!()
                    }
                }
            }
            ExprData::Bracket(ref array, ref index) => {
                let index = self.translate_expr(index);
                let array = self.translate_expr(array);
                self.builder.array_index(array, index)
            }
            ExprData::FunCall(ref ident, ref args) => match ident.content {
                FunIdentifierData::TypeSpecifier(ref type_spec) => {
                    let ty = type_specifier_to_type_desc(self.builder.module, type_spec, None);
                    let mut components = vec![];
                    for expr in args {
                        let component = self.translate_expr(expr);
                        components.push(component);
                    }
                    self.builder.construct(ty, &components)
                }
                FunIdentifierData::Expr(_) => {
                    todo!()
                }
            },
            ExprData::Dot(_, _) => {
                todo!()
            }
            ExprData::PostInc(ref expr) => {
                let place = self.translate_place(expr);
                self.builder.post_increment(place)
            }
            ExprData::PostDec(ref expr) => {
                let place = self.translate_place(expr);
                self.builder.post_increment(place)
            }
            ExprData::Comma(_, _) => {
                todo!()
            }
        }
    }

    fn translate_initializer(&mut self, initializer: &Initializer) -> Id<ast::Expr> {
        match initializer.content {
            InitializerData::Simple(ref expr) => self.translate_expr(expr),
            InitializerData::List(_) => {
                todo!()
            }
        }
    }

    fn translate_init_declarator_list(&mut self, init_declarator_list: &InitDeclaratorList) {
        let head = &init_declarator_list.head;
        let mut constant = false;
        if let Some(ref qualifier) = head.ty.qualifier {
            for qual in qualifier.qualifiers.iter() {
                match qual.content {
                    TypeQualifierSpecData::Storage(ref storage_qualifier) => match storage_qualifier.content {
                        StorageQualifierData::Const => {
                            constant = true;
                        }
                        StorageQualifierData::InOut => {}
                        StorageQualifierData::In => {}
                        StorageQualifierData::Out => {}
                        StorageQualifierData::Centroid => {}
                        StorageQualifierData::Patch => {}
                        StorageQualifierData::Sample => {}
                        StorageQualifierData::Uniform => {}
                        StorageQualifierData::Buffer => {}
                        StorageQualifierData::Shared => {}
                        StorageQualifierData::Coherent => {}
                        StorageQualifierData::Volatile => {}
                        StorageQualifierData::Restrict => {}
                        StorageQualifierData::ReadOnly => {}
                        StorageQualifierData::WriteOnly => {}
                        StorageQualifierData::Attribute => {}
                        StorageQualifierData::Varying => {}
                        StorageQualifierData::Subroutine(_) => {}
                    },
                    TypeQualifierSpecData::Layout(_) => {}
                    TypeQualifierSpecData::Precision(_) => {}
                    TypeQualifierSpecData::Interpolation(_) => {}
                    TypeQualifierSpecData::Invariant => {}
                    TypeQualifierSpecData::Precise => {}
                }
            }
        }

        let ty = type_specifier_to_type_desc(
            &mut self.builder.module,
            &head.ty.ty.content,
            head.array_specifier.as_deref(),
        );

        if let Some(ref name) = head.name {
            let init = if let Some(ref initializer) = head.initializer {
                Some(self.translate_initializer(initializer))
            } else {
                None
            };
            self.builder.emit(ast::Expr::LocalVariable {
                name: Some(name.as_str().into()),
                ty,
                init,
            });

            for tail_decl in init_declarator_list.tail.iter() {
                let init = if let Some(ref initializer) = tail_decl.initializer {
                    Some(self.translate_initializer(initializer))
                } else {
                    None
                };
                self.builder.emit(ast::Expr::LocalVariable {
                    name: Some(tail_decl.ident.ident.as_str().into()),
                    ty,
                    init,
                });
            }
        }
    }

    fn translate_statement(&mut self, statement: &Statement) {
        match statement.content {
            StatementData::Declaration(ref declaration) => match declaration.content {
                DeclarationData::FunctionPrototype(_) => {
                    todo!()
                }
                DeclarationData::InitDeclaratorList(ref decl_list) => {
                    self.translate_init_declarator_list(decl_list);
                }
                DeclarationData::Precision(_, _) => {
                    todo!()
                }
                DeclarationData::Block(_) => {
                    todo!()
                }
                DeclarationData::Invariant(_) => {
                    todo!()
                }
            },
            StatementData::Expression(ref expr) => {
                if let Some(ref expr) = expr.0 {
                    self.translate_expr(expr);
                }
            }
            StatementData::Selection(ref selection) => {
                let condition = self.translate_expr(&selection.cond);
                self.builder.if_(condition);
                match selection.rest.content {
                    SelectionRestStatementData::Statement(ref statement) => {
                        self.translate_statement(statement);
                    }
                    SelectionRestStatementData::Else(ref then_branch, ref else_branch) => {
                        self.translate_statement(then_branch);
                        self.builder.else_();
                        self.translate_statement(else_branch);
                    }
                }
                self.builder.end_if();
            }
            StatementData::Switch(_) => {}
            StatementData::CaseLabel(_) => {}
            StatementData::Iteration(ref iteration) => match iteration.content {
                IterationStatementData::While(ref condition, ref body) => {
                    self.builder.loop_();
                    let condition = match condition.content {
                        ConditionData::Expr(ref expr) => self.translate_expr(expr),
                        ConditionData::Assignment(_, _, _) => {
                            todo!()
                        }
                    };
                    let not_condition = self.builder.not(condition);
                    self.builder.if_(not_condition);
                    self.builder.break_();
                    self.builder.end_if();
                    self.translate_statement(body);
                    self.builder.end_loop();
                }
                IterationStatementData::DoWhile(ref body, ref condition) => {
                    self.builder.loop_();
                    self.translate_statement(body);
                    let condition = self.translate_expr(condition);
                    let not_condition = self.builder.not(condition);
                    self.builder.if_(not_condition);
                    self.builder.break_();
                    self.builder.end_if();
                    self.builder.end_loop();
                }
                IterationStatementData::For(ref init, ref rest, ref body) => {
                    match init.content {
                        ForInitStatementData::Expression(ref expr) => {
                            if let Some(expr) = expr {
                                self.translate_expr(expr);
                            }
                        }
                        ForInitStatementData::Declaration(ref decl) => match decl.content {
                            DeclarationData::InitDeclaratorList(ref init_decl_list) => {
                                self.translate_init_declarator_list(init_decl_list);
                            }
                            _ => {
                                panic!("invalid declaration")
                            }
                        },
                    };

                    self.builder.loop_();
                    if let Some(ref condition) = rest.condition {
                        let condition = match condition.content {
                            ConditionData::Expr(ref expr) => self.translate_expr(expr),
                            ConditionData::Assignment(_, _, _) => {
                                todo!()
                            }
                        };
                        let not_condition = self.builder.not(condition);
                        self.builder.if_(not_condition);
                        self.builder.break_();
                        self.builder.end_if();
                    }
                    self.translate_statement(body);
                    if let Some(ref post_expr) = rest.post_expr {
                        self.translate_expr(post_expr);
                    }
                    self.builder.end_loop();
                }
            },
            StatementData::Jump(ref jump) => match jump.content {
                JumpStatementData::Continue => {
                    self.builder.continue_();
                }
                JumpStatementData::Break => {
                    self.builder.break_();
                }
                JumpStatementData::Return(ref result) => {
                    let value = if let Some(ref result) = result {
                        let value = self.translate_expr(result);
                        Some(value)
                    } else {
                        None
                    };
                    self.builder.return_(value);
                }
                JumpStatementData::Discard => {
                    self.builder.discard();
                }
            },
            StatementData::Compound(ref compound_statement) => self.translate_compound_statement(compound_statement),
        }
    }

    fn translate_compound_statement(&mut self, compound_statement: &CompoundStatement) {
        for stmt in compound_statement.statement_list.iter() {
            self.translate_statement(stmt);
        }
    }
}

struct GlslTranslator<'a> {
    errors: Vec<String>,
    module: &'a mut ast::Module,
}

impl<'a> GlslTranslator<'a> {
    fn translate_function_definition(&mut self, function_definition: &FunctionDefinition) {
        let name = function_definition.prototype.name.as_str();
        let mut function_builder = self.module.build_function(name);
        let mut root_scope = Scope::new();

        for param in function_definition.prototype.parameters.iter() {
            match param.content {
                FunctionParameterDeclarationData::Named(ref type_qualifier, ref declarator) => {
                    let mut input = false;
                    let mut output = false;
                    if let Some(qual) = type_qualifier {
                        for qual in qual.qualifiers.iter() {
                            match qual.content {
                                TypeQualifierSpecData::Storage(ref storage_qualifier) => {
                                    match storage_qualifier.content {
                                        StorageQualifierData::InOut => {
                                            input = true;
                                            output = true;
                                        }
                                        StorageQualifierData::In => {
                                            input = true;
                                        }
                                        StorageQualifierData::Out => {
                                            output = true;
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {
                                    panic!("unexpected type qualifier")
                                }
                            }
                        }
                    }
                    let ty = type_specifier_to_type_desc(
                        &mut function_builder.module,
                        &declarator.ty.content,
                        declarator.ident.array_spec.as_deref(),
                    );

                    let name = SmolStr::from(declarator.ident.ident.as_str());
                    let arg = function_builder.argument(name.clone(), ty);
                    root_scope.insert(name.clone(), arg);
                }
                FunctionParameterDeclarationData::Unnamed(_, _) => {
                    todo!("unnamed function parameters")
                }
            }
        }

        let mut translator = FunctionBodyTranslator {
            builder: function_builder,
            errors: &mut self.errors,
            scopes: vec![root_scope],
        };
        translator.translate_compound_statement(&function_definition.statement);
        translator.builder.finish();
    }

    fn translate_translation_unit(&mut self, translation_unit: &TranslationUnit) {
        // external declarations are converted to shader inputs and outputs

        //let mut inputs = Vec::new();
        //let mut outputs = Vec::new();

        // process external declarations
        for decl in translation_unit.0.iter() {
            match decl.content {
                ExternalDeclarationData::Declaration(ref decl) => {
                    match decl.content {
                        DeclarationData::InitDeclaratorList(ref declarator_list) => {
                            let decl = &declarator_list.content.head;
                            let span = decl.span.unwrap();

                            let mut constant = false;
                            let mut input = false;
                            let mut output = false;
                            let mut uniform = false;

                            if let Some(ref qualifier) = decl.ty.qualifier {
                                for qual in qualifier.qualifiers.iter() {
                                    match qual.content {
                                        TypeQualifierSpecData::Storage(ref storage_qualifier) => {
                                            match storage_qualifier.content {
                                                StorageQualifierData::Const => {
                                                    constant = true;
                                                }
                                                StorageQualifierData::InOut => {}
                                                StorageQualifierData::In => {
                                                    input = true;
                                                }
                                                StorageQualifierData::Out => {
                                                    output = true;
                                                }
                                                StorageQualifierData::Centroid => {}
                                                StorageQualifierData::Patch => {}
                                                StorageQualifierData::Sample => {}
                                                StorageQualifierData::Uniform => {
                                                    uniform = true;
                                                }
                                                StorageQualifierData::Buffer => {}
                                                StorageQualifierData::Shared => {}
                                                StorageQualifierData::Coherent => {}
                                                StorageQualifierData::Volatile => {}
                                                StorageQualifierData::Restrict => {}
                                                StorageQualifierData::ReadOnly => {}
                                                StorageQualifierData::WriteOnly => {}
                                                StorageQualifierData::Attribute => {}
                                                StorageQualifierData::Varying => {}
                                                StorageQualifierData::Subroutine(_) => {}
                                            }
                                        }
                                        TypeQualifierSpecData::Layout(_) => {}
                                        TypeQualifierSpecData::Precision(_) => {}
                                        TypeQualifierSpecData::Interpolation(_) => {}
                                        TypeQualifierSpecData::Invariant => {}
                                        TypeQualifierSpecData::Precise => {}
                                    }
                                }
                            }

                            let ty = type_specifier_to_type_desc(
                                &mut self.module,
                                &decl.ty.ty.content,
                                decl.array_specifier.as_deref(),
                            );

                            let is_interface = uniform || input || output;

                            if let Some(ref name) = decl.name {
                                /*if uniform || input {
                                    inputs.push(crate::Variable::new(name.as_str(), ty));
                                } else if output {
                                    outputs.push(crate::Variable::new(name.as_str(), ty));
                                }

                                for tail_decl in declarator_list.tail.iter() {
                                    let ty = type_ctx.type_specifier_to_type_desc(
                                        &decl.ty.ty.content,
                                        tail_decl.ident.array_spec.as_deref(),
                                    )?;

                                    if uniform || input {
                                        inputs
                                            .push(crate::Variable::new(tail_decl.ident.ident.as_str(), ty));
                                    } else if output {
                                        outputs
                                            .push(crate::Variable::new(tail_decl.ident.ident.as_str(), ty));
                                    }
                                }*/
                            }
                        }
                        DeclarationData::FunctionPrototype(ref proto) => {
                            //decl_map.insert(proto.content.name.to_string(), proto.span.unwrap());
                        }
                        DeclarationData::Precision(_, _) => {}
                        DeclarationData::Block(_) => {}
                        DeclarationData::Invariant(_) => {}
                    }
                }
                ExternalDeclarationData::FunctionDefinition(ref def) => {
                    self.translate_function_definition(def);
                    /*if def.prototype.name.as_str() == "main" {
                    } else {
                    }*/
                }
                _ => {}
            }
        }
    }

    fn translate_glsl(&mut self, source: &str, source_id: &str, pp: &mut Preprocessor) {
        // setup preprocessor and construct the lexer input
        let input_file = pp.open_source(source, "").with_state(
            ProcessorState::builder()
                .extension(ext_name!("GL_GOOGLE_include_directive"), ExtensionBehavior::Enable)
                .finish(),
        );

        // parse the GLSL into a translation unit
        let mut translation_unit = TranslationUnit::parse_with_options::<Lexer<Vfs>>(
            input_file,
            &ParseOptions {
                target_vulkan: true,
                ..Default::default()
            },
        )
        .unwrap();

        self.translate_translation_unit(&translation_unit.0);
    }
}

pub fn translate_glsl(
    module: &mut ast::Module,
    source: &str,
    source_id: &str,
    pp: &mut Preprocessor,
) -> Result<(), Vec<String>> {
    let mut translator = GlslTranslator { module, errors: vec![] };
    translator.translate_glsl(source, source_id, pp);
    if !translator.errors.is_empty() {
        Err(translator.errors)
    } else {
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::{
        ast,
        glsl::{translate_glsl, GlslTranslator, Preprocessor, Vfs},
    };
    use glsl_lang::{
        lexer::v2_full::fs::{Lexer, PreprocessorExt},
        parse::Parse,
    };
    use glsl_lang_pp::processor::fs::Processor;
    use std::{
        borrow::Cow,
        collections::HashMap,
        fmt::Write,
        path::{Path, PathBuf},
    };

    // language=glsl
    const GLSL_COMMON: &str = r#"
    float sq(float x) {
        return x * x;
    }
    vec2 sq(vec2 x) {
        return x * x;
    }

    struct Foo {
        float x;
        vec2 y;
    };
    "#;

    // language=glsl
    const GLSL_SOURCE_1: &str = r#"
        #include "common.glsl" 

        in vec3 position;               // inferred variability
        in vec3 normals;                // inferred variability
        uniform in vec3 param;          // explicit uniform variability
        uniform in vec3 param2[45];     // explicit uniform variability
        out vec4 normals = f(position); // inferred variability

        uniform in sampler tex1;
        uniform in image2DMS tex2;

        uniform sampler sampler1;
        uniform texture2D tex3;

        struct MyStruct { Foo b; };
        uniform buffer MyStruct payloads[];


        vec4 f() {
            if (position) {
                return vec4(4.0, 0.0, 0.0, 1.0);
            } else {
                return vec4(0.0);
            }
        }
"#;

    // language=glsl
    const INLINE_EXPR: &str = r#"
    in vec3 color;
    out vec3 o_color = color * 0.5;

    "#;

    // language=glsl
    const STROKE_GLSL: &str = r#"
    struct Stroke {
        vec2 direction;
    };
    struct Bucket {
        int[4] values;
    };
    "#;

    // language=glsl
    const RUNTIME_ARRAYS: &str = r#"
    #include <stroke.glsl>
    
    uniform int strokeCount;
    uniform Stroke[] strokes;

    out Bucket[] buckets; 
    "#;

    #[test]
    fn test_glsl_frontend() {
        let mut vfs = Vfs::new();
        vfs.register_source("common.glsl", GLSL_COMMON);
        vfs.register_source("stroke.glsl", STROKE_GLSL);
        let mut pp = Preprocessor::new_with_fs(vfs);

        let mut module = ast::Module::new();
        translate_glsl(&mut module, GLSL_SOURCE_1, "source_1.glsl", &mut pp).unwrap();
        eprintln!("module: \n{module:#?}");
    }

    /*#[test]
    fn test_codegen() {
        let prog = make_program();
        let mut cg_shader = String::new();
        prog.codegen_decl(&mut cg_shader);
        writeln!(&mut cg_shader, "void main() {{");
        prog.codegen_main(&mut cg_shader);
        writeln!(&mut cg_shader, "}}");

        eprintln!("{}", cg_shader);
    }*/
}
