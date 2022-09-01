//! Parsing of shader programs (GLSL snippets).
use crate::{
    eval::{
        pipeline::{PipelineError, TypeDesc},
        Variability,
    },
    model::typedesc::{Field, ImageDimension, ImageType, PrimitiveType, SampledImageType, StructType},
};
use bitflags::bitflags;
use glsl_lang::{
    ast,
    ast::{
        ArraySpecifier, ArraySpecifierDimension, ArrayedIdentifier, AssignmentOp, BinaryOp, Block, CaseLabel,
        CompoundStatement, Condition, Declaration, DeclarationData, Expr, ExprStatement, ExternalDeclaration,
        ExternalDeclarationData, ForInitStatement, ForRestStatement, FullySpecifiedType, FunIdentifier,
        FunIdentifierData, FunctionDefinition, FunctionParameterDeclaration, FunctionParameterDeclarator,
        FunctionPrototype, Identifier, InitDeclaratorList, Initializer, InterpolationQualifier, IterationStatement,
        JumpStatement, LayoutQualifier, LayoutQualifierSpec, NodeSpan, PrecisionQualifier, PreprocessorDefine,
        PreprocessorElseIf, PreprocessorError, PreprocessorExtension, PreprocessorExtensionBehavior,
        PreprocessorExtensionName, PreprocessorIf, PreprocessorIfDef, PreprocessorIfNDef, PreprocessorInclude,
        PreprocessorLine, PreprocessorPragma, PreprocessorUndef, PreprocessorVersion, PreprocessorVersionProfile,
        SelectionRestStatement, SelectionStatement, SingleDeclaration, SingleDeclarationNoType, Statement,
        StorageQualifier, StorageQualifierData, StructFieldSpecifier, StructSpecifier, SwitchStatement,
        TranslationUnit, TypeName, TypeQualifier, TypeQualifierSpec, TypeQualifierSpecData, TypeSpecifier,
        TypeSpecifierNonArray, UnaryOp,
    },
    lexer::v2_full::fs::{FileSystem, Lexer, PreprocessorExt},
    parse::{Parse, ParseOptions},
    transpiler::{
        glsl,
        glsl::{
            show_array_spec, show_arrayed_identifier, show_expr, show_function_prototype, show_init_declarator_list,
            show_initializer, show_storage_qualifier, show_translation_unit, show_type_specifier, FormattingState,
        },
    },
    visitor::{HostMut, Visit},
};
use glsl_lang_pp::{
    ext_name,
    processor::{fs::Processor, nodes::ExtensionBehavior, ProcessorState},
};
use imbl::{HashMap, HashSet};
use kyute_common::Atom;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    borrow::Cow,
    cell::RefCell,
    fmt,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProgramError
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Error)]
pub enum ProgramError {
    /// The program was not syntactically valid.
    #[error("parse error(s)")]
    ParseError {
        file_id: Option<u32>,
        line: u32,
        col: u32,
        message: String,
    },
    /// There was an error in the program interface.
    #[error("program interface error: ({span:?}) {message}")]
    Interface {
        span: Option<ast::NodeSpan>,
        message: String,
    },
}

impl ProgramError {
    pub fn parse_error(file_id: u32, line: u32, col: u32, message: impl Into<String>) -> ProgramError {
        ProgramError::ParseError {
            file_id: Some(file_id),
            line,
            col,
            message: message.into(),
        }
    }

    pub fn interface(span: Option<ast::NodeSpan>, message: impl Into<String>) -> ProgramError {
        ProgramError::Interface {
            span,
            message: message.into(),
        }
    }
}

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
fn apply_array_specifier(elem_ty: &TypeDesc, array_spec: &ast::ArraySpecifierData) -> Result<TypeDesc, ProgramError> {
    let mut ty = elem_ty.clone();
    for dim in array_spec.dimensions.iter() {
        match dim.content {
            ast::ArraySpecifierDimensionData::Unsized => {
                ty = TypeDesc::RuntimeArray(Arc::new(ty));
            }
            ast::ArraySpecifierDimensionData::ExplicitlySized(ref expr) => match expr.content {
                ast::ExprData::IntConst(value) => {
                    if value < 0 {
                        return Err(ProgramError::interface(
                            expr.span,
                            "array size must be non-negative".to_string(),
                        ));
                    }
                    ty = TypeDesc::Array {
                        elem_ty: Arc::new(ty),
                        len: value as u32,
                    };
                }
                ast::ExprData::UIntConst(value) => {
                    ty = TypeDesc::Array {
                        elem_ty: Arc::new(ty),
                        len: value,
                    };
                }
                _ => {
                    return Err(ProgramError::interface(
                        expr.span,
                        "unsupported array length expression in program interface".to_string(),
                    ))
                }
            },
        }
    }
    Ok(ty)
}

#[derive(Clone, Debug)]
struct UserType {
    span: NodeSpan,
    ty: TypeDesc,
}

/// Map of user-defined types in the current source file.
struct TypeCtx {
    user_types: HashMap<String, UserType>,
}

impl TypeCtx {
    fn new() -> Self {
        Self {
            user_types: Default::default(),
        }
    }

    /// Converts a GLSL type specifier into a `TypeDesc`.
    fn type_specifier_to_type_desc(
        &mut self,
        spec: &ast::TypeSpecifierData,
        array_spec: Option<&ast::ArraySpecifierData>,
    ) -> Result<TypeDesc, ProgramError> {
        let mut ty = match spec.ty.content {
            ast::TypeSpecifierNonArrayData::Void => TypeDesc::Void,
            ast::TypeSpecifierNonArrayData::Bool => TypeDesc::Primitive(PrimitiveType::Bool),
            ast::TypeSpecifierNonArrayData::Int => TypeDesc::Primitive(PrimitiveType::Int),
            ast::TypeSpecifierNonArrayData::UInt => TypeDesc::Primitive(PrimitiveType::UnsignedInt),
            ast::TypeSpecifierNonArrayData::Float => TypeDesc::Primitive(PrimitiveType::Float),
            ast::TypeSpecifierNonArrayData::Double => TypeDesc::Primitive(PrimitiveType::Double),
            ast::TypeSpecifierNonArrayData::Vec2 => TypeDesc::Vector {
                len: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Vec3 => TypeDesc::Vector {
                len: 3,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Vec4 => TypeDesc::Vector {
                len: 4,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::DVec2 => TypeDesc::Vector {
                len: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DVec3 => TypeDesc::Vector {
                len: 3,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DVec4 => TypeDesc::Vector {
                len: 4,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::BVec2 => TypeDesc::Vector {
                len: 2,
                elem_ty: PrimitiveType::Bool,
            },
            ast::TypeSpecifierNonArrayData::BVec3 => TypeDesc::Vector {
                len: 3,
                elem_ty: PrimitiveType::Bool,
            },
            ast::TypeSpecifierNonArrayData::BVec4 => TypeDesc::Vector {
                len: 4,
                elem_ty: PrimitiveType::Bool,
            },
            ast::TypeSpecifierNonArrayData::IVec2 => TypeDesc::Vector {
                len: 2,
                elem_ty: PrimitiveType::Int,
            },
            ast::TypeSpecifierNonArrayData::IVec3 => TypeDesc::Vector {
                len: 3,
                elem_ty: PrimitiveType::Int,
            },
            ast::TypeSpecifierNonArrayData::IVec4 => TypeDesc::Vector {
                len: 4,
                elem_ty: PrimitiveType::Int,
            },
            ast::TypeSpecifierNonArrayData::UVec2 => TypeDesc::Vector {
                len: 2,
                elem_ty: PrimitiveType::UnsignedInt,
            },
            ast::TypeSpecifierNonArrayData::UVec3 => TypeDesc::Vector {
                len: 3,
                elem_ty: PrimitiveType::UnsignedInt,
            },
            ast::TypeSpecifierNonArrayData::UVec4 => TypeDesc::Vector {
                len: 4,
                elem_ty: PrimitiveType::UnsignedInt,
            },
            ast::TypeSpecifierNonArrayData::Mat2 => TypeDesc::Matrix {
                rows: 2,
                columns: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat3 => TypeDesc::Matrix {
                rows: 3,
                columns: 3,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat4 => TypeDesc::Matrix {
                rows: 4,
                columns: 4,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat22 => TypeDesc::Matrix {
                rows: 2,
                columns: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat23 => TypeDesc::Matrix {
                rows: 3,
                columns: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat24 => TypeDesc::Matrix {
                rows: 4,
                columns: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat32 => TypeDesc::Matrix {
                rows: 2,
                columns: 3,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat33 => TypeDesc::Matrix {
                rows: 3,
                columns: 3,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat34 => TypeDesc::Matrix {
                rows: 3,
                columns: 4,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat42 => TypeDesc::Matrix {
                rows: 4,
                columns: 2,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat43 => TypeDesc::Matrix {
                rows: 4,
                columns: 3,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::Mat44 => TypeDesc::Matrix {
                rows: 4,
                columns: 4,
                elem_ty: PrimitiveType::Float,
            },
            ast::TypeSpecifierNonArrayData::DMat2 => TypeDesc::Matrix {
                rows: 2,
                columns: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat3 => TypeDesc::Matrix {
                rows: 3,
                columns: 3,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat4 => TypeDesc::Matrix {
                rows: 4,
                columns: 4,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat22 => TypeDesc::Matrix {
                rows: 2,
                columns: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat23 => TypeDesc::Matrix {
                rows: 3,
                columns: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat24 => TypeDesc::Matrix {
                rows: 4,
                columns: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat32 => TypeDesc::Matrix {
                rows: 2,
                columns: 3,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat33 => TypeDesc::Matrix {
                rows: 3,
                columns: 3,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat34 => TypeDesc::Matrix {
                rows: 3,
                columns: 4,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat42 => TypeDesc::Matrix {
                rows: 4,
                columns: 2,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat43 => TypeDesc::Matrix {
                rows: 4,
                columns: 3,
                elem_ty: PrimitiveType::Double,
            },
            ast::TypeSpecifierNonArrayData::DMat44 => TypeDesc::Matrix {
                rows: 4,
                columns: 4,
                elem_ty: PrimitiveType::Double,
            },

            ast::TypeSpecifierNonArrayData::Texture1D => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim1D, false)
            }
            ast::TypeSpecifierNonArrayData::Texture2D => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim2D, false)
            }
            ast::TypeSpecifierNonArrayData::Texture3D => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim3D, false)
            }
            ast::TypeSpecifierNonArrayData::TextureCube => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::DimCube, false)
            }
            //ast::TypeSpecifierNonArrayData::Texture2DRect => {tyctx.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim2DRect, false)}
            ast::TypeSpecifierNonArrayData::Texture1DArray => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim1DArray, false)
            }
            ast::TypeSpecifierNonArrayData::Texture2DArray => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim2DArray, false)
            }
            //ast::TypeSpecifierNonArrayData::TextureBuffer => {}
            ast::TypeSpecifierNonArrayData::Texture2DMs => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim2D, true)
            }
            ast::TypeSpecifierNonArrayData::Texture2DMsArray => {
                self.define_sampled_image_type(PrimitiveType::Float, ImageDimension::Dim2DArray, true)
            }
            //ast::TypeSpecifierNonArrayData::TextureCubeArray => {tyctx.define_sampled_image_type(PrimitiveType::Float, ImageDimension::DimCubeArray, false)}
            ast::TypeSpecifierNonArrayData::ITexture1D => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim1D, false)
            }
            ast::TypeSpecifierNonArrayData::ITexture2D => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim2D, false)
            }
            ast::TypeSpecifierNonArrayData::ITexture3D => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim3D, false)
            }
            ast::TypeSpecifierNonArrayData::ITextureCube => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::DimCube, false)
            }
            //ast::TypeSpecifierNonArrayData::ITexture2DRect => {}
            ast::TypeSpecifierNonArrayData::ITexture1DArray => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim1DArray, false)
            }
            ast::TypeSpecifierNonArrayData::ITexture2DArray => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim2DArray, false)
            }
            //ast::TypeSpecifierNonArrayData::ITextureBuffer => {}
            ast::TypeSpecifierNonArrayData::ITexture2DMs => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim2D, true)
            }
            ast::TypeSpecifierNonArrayData::ITexture2DMsArray => {
                self.define_sampled_image_type(PrimitiveType::Int, ImageDimension::Dim2DArray, true)
            }
            //ast::TypeSpecifierNonArrayData::ITextureCubeArray => {
            //    tyctx.define_sampled_image_type(PrimitiveType::Int, ImageDimension::DimCubeArray, false)
            //}
            ast::TypeSpecifierNonArrayData::Image1D => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim1D, false)
            }
            ast::TypeSpecifierNonArrayData::Image2D => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim2D, false)
            }
            ast::TypeSpecifierNonArrayData::Image3D => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim3D, false)
            }
            ast::TypeSpecifierNonArrayData::ImageCube => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::DimCube, false)
            }
            ast::TypeSpecifierNonArrayData::Image1DArray => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim1DArray, false)
            }
            ast::TypeSpecifierNonArrayData::Image2DArray => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim2DArray, false)
            }
            ast::TypeSpecifierNonArrayData::Image2DMs => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim2D, true)
            }
            ast::TypeSpecifierNonArrayData::Image2DMsArray => {
                self.define_image_type(PrimitiveType::Float, ImageDimension::Dim2DArray, true)
            }
            //ast::TypeSpecifierNonArrayData::ImageCubeArray => {
            //    tyctx.define_image_type(PrimitiveType::Float, ImageDimension::DimCubeArray, false)
            //}
            ast::TypeSpecifierNonArrayData::IImage1D => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim1D, false)
            }
            ast::TypeSpecifierNonArrayData::IImage2D => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim2D, false)
            }
            ast::TypeSpecifierNonArrayData::IImage3D => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim3D, false)
            }
            ast::TypeSpecifierNonArrayData::IImageCube => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::DimCube, false)
            }
            ast::TypeSpecifierNonArrayData::IImage1DArray => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim1DArray, false)
            }
            ast::TypeSpecifierNonArrayData::IImage2DArray => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim2DArray, false)
            }
            ast::TypeSpecifierNonArrayData::IImage2DMs => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim2D, true)
            }
            ast::TypeSpecifierNonArrayData::IImage2DMsArray => {
                self.define_image_type(PrimitiveType::Int, ImageDimension::Dim2DArray, true)
            }
            //ast::TypeSpecifierNonArrayData::IImageCubeArray => {
            //    tyctx.define_image_type(PrimitiveType::Int, ImageDimension::DimCubeArray, false)
            //}
            ast::TypeSpecifierNonArrayData::AtomicUInt => todo!(),
            ast::TypeSpecifierNonArrayData::UImage1D => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim1D, false)
            }
            ast::TypeSpecifierNonArrayData::UImage2D => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim2D, false)
            }
            ast::TypeSpecifierNonArrayData::UImage3D => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim3D, false)
            }
            //ast::TypeSpecifierNonArrayData::UImageCube => {
            //    tyctx.define_image_type(PrimitiveType::UInt, ImageDimension::DimCube, false)
            //}
            ast::TypeSpecifierNonArrayData::UImage1DArray => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim1DArray, false)
            }
            ast::TypeSpecifierNonArrayData::UImage2DArray => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim2DArray, false)
            }
            //ast::TypeSpecifierNonArrayData::UImageBuffer => {
            //    tyctx.define_image_type(PrimitiveType::UInt, ImageDimension::DimBuffer, false)
            //}
            ast::TypeSpecifierNonArrayData::UImage2DMs => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim2D, true)
            }
            ast::TypeSpecifierNonArrayData::UImage2DMsArray => {
                self.define_image_type(PrimitiveType::UnsignedInt, ImageDimension::Dim2DArray, true)
            }
            ast::TypeSpecifierNonArrayData::Sampler => TypeDesc::Sampler,
            ast::TypeSpecifierNonArrayData::SamplerShadow => TypeDesc::ShadowSampler,
            //ast::TypeSpecifierNonArrayData::UImageCubeArray => {
            //    tyctx.define_image_type(PrimitiveType::UInt, ImageDimension::DimCubeArray, false)
            //}
            ast::TypeSpecifierNonArrayData::Struct(ref s) => {
                // parse struct type
                let name = s.content.name.as_ref().map(|name| name.content.0.to_string());

                let mut fields = Vec::new();

                for f in s.content.fields.iter() {
                    for ident in f.identifiers.iter() {
                        let field_name = ident.content.ident.content.0.to_string();
                        let field_ty = self.type_specifier_to_type_desc(
                            &f.ty.content,
                            ident.content.array_spec.as_ref().map(|spec| &spec.content),
                        )?;

                        fields.push(Field {
                            name: field_name.into(),
                            ty: field_ty,
                        })
                    }
                }

                let span = s.span.unwrap();

                let ty = TypeDesc::Struct(Arc::new(StructType {
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
                }));

                if let Some(name) = name {
                    self.define_struct_type(name, span, ty.clone())?;
                }

                ty
            }
            ast::TypeSpecifierNonArrayData::TypeName(ref name) => {
                let ty = self.user_type(name.content.as_str()).ok_or_else(|| {
                    ProgramError::interface(name.span, format!("unknown type `{}` in interface", name.content.0))
                })?;
                ty.ty
            }

            _ => {
                return Err(ProgramError::interface(
                    spec.ty.span,
                    format!("unsupported type `{:?}` in interface", spec.ty.content),
                ));
            }
        };

        // array specifier attached to the type
        if let Some(ref array_spec) = spec.array_specifier {
            ty = apply_array_specifier(&ty, &array_spec.content)?;
        }

        // array specified attached to the identifier
        if let Some(ref array_spec) = array_spec {
            ty = apply_array_specifier(&ty, array_spec)?;
        }

        Ok(ty)
    }

    fn define_array_type(&mut self, element_ty: impl Into<Arc<TypeDesc>>, size: usize) -> TypeDesc {
        TypeDesc::Array {
            elem_ty: element_ty.into(),
            len: size as u32,
        }
    }

    fn define_runtime_array_type(&mut self, element_ty: &TypeDesc) -> TypeDesc {
        TypeDesc::RuntimeArray(Arc::new(element_ty.clone()))
    }

    fn define_sampled_image_type(&mut self, sampled_ty: PrimitiveType, dim: ImageDimension, ms: bool) -> TypeDesc {
        TypeDesc::SampledImage(Arc::new(SampledImageType { sampled_ty, dim, ms }))
    }

    fn define_image_type(&mut self, element_ty: PrimitiveType, dim: ImageDimension, ms: bool) -> TypeDesc {
        TypeDesc::Image(Arc::new(ImageType { element_ty, dim, ms }))
    }

    fn define_struct_type(&mut self, name: String, span: NodeSpan, ty: TypeDesc) -> Result<(), ProgramError> {
        self.user_types.entry(name).or_insert_with(|| UserType { span, ty });
        Ok(())
    }

    fn user_type(&self, name: &str) -> Option<UserType> {
        self.user_types.get(name).cloned()
    }

    fn dump(&self) {
        eprintln!("User types: ");
        for (name, ty) in self.user_types.iter() {
            eprintln!(" - {} : {:?}", name, ty.span);
        }
        eprintln!();
    }
}

bitflags! {
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
}

fn get_storage_qualifiers(ty: &ast::FullySpecifiedTypeData) -> StorageQualifiers {
    let mut qualifiers = StorageQualifiers::empty();
    if let Some(ref ty_qualifier_data) = ty.qualifier {
        for qual in ty_qualifier_data.content.qualifiers.iter() {
            match qual.content {
                ast::TypeQualifierSpecData::Storage(ref data) => match data.content {
                    ast::StorageQualifierData::Const => {
                        qualifiers |= StorageQualifiers::CONST;
                    }
                    ast::StorageQualifierData::InOut => {
                        qualifiers |= StorageQualifiers::INOUT;
                    }
                    ast::StorageQualifierData::In => {
                        qualifiers |= StorageQualifiers::IN;
                    }
                    ast::StorageQualifierData::Out => {
                        qualifiers |= StorageQualifiers::OUT;
                    }
                    ast::StorageQualifierData::Centroid => {
                        qualifiers |= StorageQualifiers::CENTROID;
                    }
                    ast::StorageQualifierData::Patch => {
                        qualifiers |= StorageQualifiers::PATCH;
                    }
                    ast::StorageQualifierData::Sample => {
                        qualifiers |= StorageQualifiers::SAMPLE;
                    }
                    ast::StorageQualifierData::Uniform => {
                        qualifiers |= StorageQualifiers::UNIFORM;
                    }
                    ast::StorageQualifierData::Buffer => {
                        qualifiers |= StorageQualifiers::BUFFER;
                    }
                    ast::StorageQualifierData::Shared => {
                        qualifiers |= StorageQualifiers::SHARED;
                    }
                    ast::StorageQualifierData::Coherent => {
                        qualifiers |= StorageQualifiers::COHERENT;
                    }
                    ast::StorageQualifierData::Volatile => {
                        qualifiers |= StorageQualifiers::VOLATILE;
                    }
                    ast::StorageQualifierData::Restrict => {
                        qualifiers |= StorageQualifiers::RESTRICT;
                    }
                    ast::StorageQualifierData::ReadOnly => {
                        qualifiers |= StorageQualifiers::READONLY;
                    }
                    ast::StorageQualifierData::WriteOnly => {
                        qualifiers |= StorageQualifiers::WRITEONLY;
                    }
                    ast::StorageQualifierData::Attribute => {
                        qualifiers |= StorageQualifiers::ATTRIBUTE;
                    }
                    ast::StorageQualifierData::Varying => {
                        qualifiers |= StorageQualifiers::VARYING;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    qualifiers
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Program
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Input or output variable of a program
#[derive(Clone, Debug)]
pub struct ProgramInterface {
    /// Input name
    pub name: Arc<str>,
    /// Input type
    pub ty: TypeDesc,
    /// Source ID.
    pub source_id: u32,
    /// Explicit variability
    pub variability: Option<Variability>,
    /// Whether this is an output of the program.
    pub output: bool,
}

/// A program, taking a set of values as input and producing others as a result.
///
/// These are composed to create GPU pipelines.
#[derive(Clone, Debug)]
pub struct Program {
    /// Parsed GLSL translation unit.
    pub(crate) translation_unit: ast::TranslationUnit,
    /// Inputs & outputs of the program.
    pub(crate) interface: Vec<ProgramInterface>,
}

impl Program {
    /// Creates a new program from GLSL source code.
    ///
    /// # Arguments
    ///
    /// * `source` - GLSL source code.
    /// * `file_id` - file ID of the source.
    /// * `processor` - GLSL preprocessor instance.
    pub fn new(source: &str, source_id: impl AsRef<Path>, pp: &mut Preprocessor) -> Result<Program, ProgramError> {
        let mut decl_ctx = TypeCtx::new();

        // setup preprocessor and construct the lexer input
        let input_file = pp.open_source(source, "").with_state(
            ProcessorState::builder()
                .extension(ext_name!("GL_GOOGLE_include_directive"), ExtensionBehavior::Enable)
                .finish(),
        );

        // parse the GLSL into a translation unit
        let mut translation_unit = ast::TranslationUnit::parse_with_options::<Lexer<Vfs>>(
            input_file,
            &ParseOptions {
                target_vulkan: true,
                ..Default::default()
            },
        )
        .map_err(|err| ProgramError::ParseError {
            file_id: err.current_file().map(|id| id.number()),
            line: err.line(),
            col: err.col(),
            message: err.to_string(),
        })?
        .0;

        // map from type name to declaration site & typedesc
        let mut type_ctx = TypeCtx::new();
        // map from decl name to declaration site
        //let mut decl_map: HashMap<String, NodeSpan> = HashMap::new();

        let mut interface = Vec::new();

        // process external declarations
        for decl in translation_unit.0.iter() {
            match decl.content {
                ast::ExternalDeclarationData::Declaration(ref decl) => match decl.content {
                    ast::DeclarationData::InitDeclaratorList(ref declarator_list) => {
                        let decl = &declarator_list.content.head;
                        let span = decl.span.unwrap();
                        let storage_qualifiers = get_storage_qualifiers(&decl.ty.content);
                        let ty = type_ctx.type_specifier_to_type_desc(
                            &decl.ty.content.ty.content,
                            decl.array_specifier.as_ref().map(|x| &x.content),
                        )?;

                        if let Some(ref name) = decl.name {
                            let name = name.content.as_str();
                            if storage_qualifiers.intersects(StorageQualifiers::INTERFACE_MASK) {
                                // this is an interface variable
                                let interface_var = ProgramInterface {
                                    name: name.into(),
                                    ty: ty.clone(),
                                    source_id: span.source_id().number(),
                                    variability: None,
                                    output: storage_qualifiers.contains(StorageQualifiers::OUT),
                                };
                                interface.push(interface_var);
                            }
                            // not an interface variable, it is subject to renaming
                            //decl_map.insert(name.to_string(), span);
                        } else {
                        }
                    }
                    ast::DeclarationData::FunctionPrototype(ref proto) => {
                        //decl_map.insert(proto.content.name.to_string(), proto.span.unwrap());
                    }
                    _ => {}
                },
                ast::ExternalDeclarationData::FunctionDefinition(ref def) => {
                    //decl_map.insert(def.prototype.name.to_string(), def.span.unwrap());
                }
                _ => {}
            }
        }

        /*// rewrite AST to "namespace" external decls with file ID appended to the name.
        let mut rewriter = AppendFileIdRewriter {
            ty_ctx: &mut type_ctx,
            decl_map: &mut decl_map,
            current_args: Default::default(),
        };

        translation_unit.visit_mut(&mut rewriter);*/

        //let mut out_str = String::new();
        //show_translation_unit(&mut out_str, &translation_unit, FormattingState::default());
        //eprintln!("{}", out_str);

        Ok(Program {
            translation_unit,
            interface,
        })
    }

    /*/// Returns the initializer node for the specified output.
    pub(crate) fn interface_initializer_mut(&mut self, interface_name: &str) -> Option<&mut ast::Initializer> {
        for decl in self.translation_unit.0.iter_mut() {
            match decl.content {
                ast::ExternalDeclarationData::Declaration(ref mut decl) => match decl.content {
                    ast::DeclarationData::InitDeclaratorList(ref mut declarator_list) => {
                        let decl = &declarator_list.content.head;

                        if let Some(ref name) = decl.name {
                            if name.as_str() == interface_name {
                                return decl.initializer.as_mut();
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        return None;
    }*/

    /// Returns an iterator over all external declarations.
    pub(crate) fn external_declarations(&self) -> impl Iterator<Item = &ExternalDeclaration> {
        self.translation_unit.0.iter()
    }

    /// Returns the interface of the program, i.e. all inputs and outputs.
    pub fn interface(&self) -> &[ProgramInterface] {
        &self.interface
    }

    /// Returns a reference to the named interface variable.
    pub fn interface_variable_by_name(&self, name: &str) -> Option<&ProgramInterface> {
        self.interface.iter().find(|var| &*var.name == name)
    }

    /// Returns the index of the named interface variable.
    pub fn interface_index(&self, name: &str) -> Option<usize> {
        self.interface.iter().position(|var| &*var.name == name)
    }

    /*pub fn inputs(&self) -> impl Iterator<Item = &ProgramInterface> {
        self.interface.values().filter(|x| !x.output)
    }

    pub fn outputs(&self) -> impl Iterator<Item = &ProgramInterface> {
        self.interface.values().filter(|x| x.output)
    }*/
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Program pipelines
////////////////////////////////////////////////////////////////////////////////////////////////////

// ProgramNode:
// - program
// - bindings
// - ancestor nodes
// - resulting variables

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::eval::pipeline::{
        program::{ProgramError, Vfs},
        Program,
    };
    use artifice::eval::pipeline::program::Preprocessor;
    use glsl_lang::{
        ast,
        lexer::v2_full::fs::{Lexer, PreprocessorExt},
        parse::Parse,
    };
    use glsl_lang_pp::processor::fs::Processor;
    use graal_spirv::spv::SourceLanguage::GLSL;
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
            return vec4(0.0,0.0,0.0,1.0);
        }
"#;

    // language=glsl
    const INLINE_EXPR: &str = r#"
    in vec3 color;
    out vec3 o_color = color * vec3(1.0, 2.0, 3.0);

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

    /*#[test]
    fn test_glsl_ast() {
        let options = glsl_lang::lexer::ParseOptions {
            target_vulkan: true,
            ..Default::default()
        };

        let mut vfs = Vfs::new();
        vfs.add("<main>", GLSL_SOURCE_1);
        let mut pp = Processor::new_with_fs(vfs);
        let input_file = pp.open("<main>").unwrap();

        let ast = ast::TranslationUnit::parse_with_options::<Lexer<Vfs>>(input_file, &options)
            .map_err(|err| ProgramError::ParseError {
                file_id: err.current_file().map(|id| id.number()),
                line: err.line(),
                col: err.col(),
                message: err.to_string(),
            })
            .map(|ast| ast.0);
        eprintln!("{:?}", ast);
        assert!(ast.is_ok());
    }*/

    fn make_program() -> Program {
        let mut vfs = Vfs::new();
        vfs.register_source("common.glsl", GLSL_COMMON);
        vfs.dump();
        let mut pp = Preprocessor::new_with_fs(vfs);
        let program = Program::new(GLSL_SOURCE_1, "/main/test", &mut pp).expect("could not parse program");
        program
    }

    #[test]
    fn test_program() {
        make_program();
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
