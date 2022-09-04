//! GLSL
//! Parsing of shader programs (GLSL snippets).
use crate::{
    ast,
    ast::{Constant, Function, GlobalVariable, Id, Module, NameResolution, TypeDesc},
};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle, Severity},
    files::Error,
    term,
    term::termcolor::{ColorChoice, StandardStream, WriteColor},
};
use glsl_lang as glsl;
use glsl_lang::{
    ast::{
        ArraySpecifierData, ArraySpecifierDimensionData, AssignmentOpData, BinaryOpData, CompoundStatement,
        ConditionData, DeclarationData, Expr, ExprData, ExternalDeclarationData, ForInitStatementData,
        FunIdentifierData, FunctionDefinition, FunctionParameterDeclarationData, Identifier, InitDeclaratorList,
        Initializer, InitializerData, IterationStatementData, JumpStatementData, Node, NodeSpan,
        SelectionRestStatementData, Statement, StatementData, StorageQualifierData, TranslationUnit,
        TypeQualifierSpecData, TypeSpecifierData, TypeSpecifierNonArrayData, UnaryOpData,
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
use indexmap::{IndexMap, IndexSet};
use lang_util::located::FileIdResolver;
use smallvec::smallvec;
use smol_str::SmolStr;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    fmt::Display,
    ops::Range,
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
struct SourceFile {
    name: SmolStr,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(name: SmolStr, text: String) -> SourceFile {
        let line_starts: Vec<_> = codespan_reporting::files::line_starts(&text).collect();
        SourceFile {
            name,
            text,
            line_starts,
        }
    }
}

#[derive(Clone)]
pub struct SourceFiles {
    files: Arc<IndexMap<SmolStr, SourceFile>>,
}

impl SourceFiles {
    /// Creates a new instance.
    pub fn new() -> SourceFiles {
        SourceFiles {
            files: Arc::new(IndexMap::new()),
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Registers a file.
    pub fn register_source(&mut self, name: impl Into<SmolStr>, src: impl Into<String>) -> usize {
        let name = name.into();
        Arc::make_mut(&mut self.files)
            .insert_full(name.clone(), SourceFile::new(name.clone(), src.into()))
            .0
    }
}

impl<'a> FileSystem for &'a SourceFiles {
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
            Ok(Cow::Borrowed(&src.text))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        }
    }
}

pub type Preprocessor<'a> = Processor<&'a SourceFiles>;

struct Files<'a> {
    main_source: &'a SourceFile,
    other_sources: &'a SourceFiles,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum FileOrSourceId {
    File(usize),
    Source,
}

impl<'a> codespan_reporting::files::Files<'a> for Files<'a> {
    type FileId = FileOrSourceId;
    type Name = SmolStr;
    type Source = &'a str;

    fn name(&'a self, id: FileOrSourceId) -> Result<Self::Name, codespan_reporting::files::Error> {
        match id {
            FileOrSourceId::Source => Ok(self.main_source.name.clone()),
            FileOrSourceId::File(index) => Ok(self.other_sources.files[index].name.clone()),
        }
    }

    fn source(&'a self, id: FileOrSourceId) -> Result<&'a str, codespan_reporting::files::Error> {
        match id {
            FileOrSourceId::Source => Ok(&self.main_source.text),
            FileOrSourceId::File(index) => Ok(&self.other_sources.files[index].text),
        }
    }

    fn line_index(&'a self, id: FileOrSourceId, byte_index: usize) -> Result<usize, codespan_reporting::files::Error> {
        let source = match id {
            FileOrSourceId::Source => self.main_source,
            FileOrSourceId::File(index) => &self.other_sources.files[index],
        };

        let line = match source.line_starts.binary_search(&byte_index) {
            Ok(line) => line,
            Err(next_line) => next_line - 1,
        };
        Ok(line)
    }

    fn line_range(
        &'a self,
        id: FileOrSourceId,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        let source = match id {
            FileOrSourceId::Source => self.main_source,
            FileOrSourceId::File(index) => &self.other_sources.files[index],
        };

        let line_start = source.line_starts[line_index];
        let next_line_start = source
            .line_starts
            .get(line_index + 1)
            .cloned()
            .unwrap_or(source.text.len());
        Ok(line_start..next_line_start)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
//
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct DiagnosticBuilder<'a, 'b> {
    sink: &'a mut DiagnosticSink<'b>,
    diag: Diagnostic<FileOrSourceId>,
}

fn span_to_range_and_file_index(
    pp: &Preprocessor,
    sources: &SourceFiles,
    span: NodeSpan,
) -> (Range<usize>, FileOrSourceId) {
    let start: usize = span.range().start().into();
    let end: usize = span.range().end().into();
    // roundabout way of getting our file index
    // we're doing FileId(glsl_lang) -> file path (name?) -> FileID (ours)
    // PP's API not super practical here, would prefer to assign file IDs ourselves via the FileSystem trait.
    let path = pp.resolve(span.source_id());
    let file_index = if let Some(path) = path {
        // hmpf...
        if let Some((index, _, _)) = sources.files.get_full(&*path.to_string_lossy()) {
            FileOrSourceId::File(index)
        } else {
            eprintln!("unknown source file: `{}`", path.display());
            FileOrSourceId::Source
        }
    } else {
        FileOrSourceId::Source
    };
    (start..end, file_index)
}

impl<'a, 'b> DiagnosticBuilder<'a, 'b> {
    fn label(
        mut self,
        span: Option<NodeSpan>,
        style: LabelStyle,
        message: impl Into<String>,
    ) -> DiagnosticBuilder<'a, 'b> {
        if let Some(span) = span {
            let (range, file_id) = span_to_range_and_file_index(self.sink.pp, self.sink.files, span);
            self.diag.labels.push(Label {
                style,
                file_id,
                range,
                message: message.into(),
            });
        } else {
            self.diag.notes.push(message.into());
        }
        self
    }

    pub fn primary_label(self, span: Option<NodeSpan>, message: impl Into<String>) -> DiagnosticBuilder<'a, 'b> {
        self.label(span, LabelStyle::Primary, message)
    }

    pub fn secondary_label(self, span: Option<NodeSpan>, message: impl Into<String>) -> DiagnosticBuilder<'a, 'b> {
        self.label(span, LabelStyle::Secondary, message)
    }

    pub fn note(mut self, message: impl Into<String>) -> DiagnosticBuilder<'a, 'b> {
        self.diag.notes.push(message.into());
        self
    }

    pub fn emit(mut self) {
        match self.diag.severity {
            Severity::Bug => {
                self.sink.bug_count += 1;
            }
            Severity::Error => {
                self.sink.error_count += 1;
            }
            Severity::Warning => {
                self.sink.warning_count += 1;
            }
            _ => {}
        }
        let files = Files {
            main_source: self.sink.main_source,
            other_sources: self.sink.files,
        };
        term::emit(self.sink.writer, &self.sink.config, &files, &self.diag).expect("diagnostic output failed")
    }
}

pub struct DiagnosticSink<'a> {
    writer: &'a mut dyn WriteColor,
    config: codespan_reporting::term::Config,
    files: &'a SourceFiles,
    main_source: &'a SourceFile,
    pp: &'a Preprocessor<'a>,
    bug_count: usize,
    error_count: usize,
    warning_count: usize,
}

impl<'a> DiagnosticSink<'a> {
    fn new(
        writer: &'a mut dyn WriteColor,
        config: codespan_reporting::term::Config,
        files: &'a SourceFiles,
        main_source: &'a SourceFile,
        pp: &'a Preprocessor<'a>,
    ) -> DiagnosticSink {
        DiagnosticSink {
            writer,
            config,
            files,
            main_source,
            pp,
            bug_count: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    pub fn bug<'b>(&'b mut self, message: impl Into<String>) -> DiagnosticBuilder<'b, 'a> {
        DiagnosticBuilder {
            sink: self,
            diag: Diagnostic::new(Severity::Bug).with_message(message.into()),
        }
    }

    pub fn error<'b>(&'b mut self, message: impl Into<String>) -> DiagnosticBuilder<'b, 'a> {
        DiagnosticBuilder {
            sink: self,
            diag: Diagnostic::new(Severity::Error).with_message(message.into()),
        }
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    pub fn bug_count(&self) -> usize {
        self.bug_count
    }
}

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
    diag: &mut DiagnosticSink,
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
                        diag,
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
        TypeSpecifierNonArrayData::TypeName(ref name) => {
            let ty = module.user_type(name.as_str());
            if ty == module.error_type {
                diag.error("unknown type")
                    .primary_label(name.span, "unknown type")
                    .emit()
            }
            ty
        }
        _ => {
            diag.bug("unsupported type")
                .primary_label(spec.ty.span, "unknown type")
                .emit();
            module.error_type
        }
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

struct ScopeStack {
    arguments: HashMap<SmolStr, u32>,
    scopes: Vec<Scope>,
}

impl ScopeStack {
    fn new(arguments: HashMap<SmolStr, u32>) -> ScopeStack {
        ScopeStack {
            arguments,
            scopes: vec![Scope::new()],
        }
    }

    fn enter(&mut self) {
        let new_scope = self.scopes.last().unwrap().clone();
        self.scopes.push(new_scope);
    }

    fn exit(&mut self) {
        self.scopes.pop().expect("unbalanced scopes");
    }

    fn find_variable(&self, name: &str) -> Option<Id<ast::Expr>> {
        self.scopes.last().unwrap().get(name).cloned()
    }

    fn declare(&mut self, name: impl Into<SmolStr>, variable: Id<ast::Expr>) {
        self.scopes.last_mut().unwrap().insert(name.into(), variable);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum LocalNameResolution {
    Local(Id<ast::Expr>),
    Argument(u32),
    Global(ast::NameResolution),
}

fn resolve_name(name: &str, m: &ast::Module, s: &ScopeStack) -> Option<LocalNameResolution> {
    if let Some(&var) = s.scopes.last().unwrap().get(name) {
        Some(LocalNameResolution::Local(var))
    } else if let Some(&index) = s.arguments.get(name) {
        Some(LocalNameResolution::Argument(index))
    } else if let Some(global) = m.resolve_name(name) {
        Some(LocalNameResolution::Global(global))
    } else {
        None
    }
}

fn translate_place(
    b: &mut ast::FunctionBuilder,
    s: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    expr: &Expr,
) -> Id<ast::Expr> {
    let span = expr.span;
    match expr.content {
        ExprData::Variable(ref var) => {
            let name = var.as_str();
            match resolve_name(name, b.module, s) {
                None => {
                    diag.error(format!("unresolved name `{name}`"))
                        .primary_label(span, "")
                        .emit();
                    b.error()
                }
                Some(LocalNameResolution::Local(var)) => {
                    // TODO: check that this is a pointer
                    var
                }
                Some(LocalNameResolution::Global(ast::NameResolution::Constant(_))) => {
                    diag.error(format!("cannot assign to a constant"))
                        .primary_label(span, "this is a constant")
                        .emit();
                    b.error()
                }
                Some(LocalNameResolution::Argument(index)) => {
                    // TODO: check that this is a pointer
                    b.argument(index)
                }
                Some(LocalNameResolution::Global(ast::NameResolution::GlobalVariable(variable))) => {
                    // TODO: check that this is a pointer and that is it assignable
                    b.global(variable)
                }
                Some(LocalNameResolution::Global(ast::NameResolution::Function(_))) => {
                    diag.error(format!("cannot assign to a function"))
                        .primary_label(span, "this is a function")
                        .emit();
                    b.error()
                }
            }
        }
        ExprData::Bracket(ref array, ref index) => {
            let array = translate_place(b, s, diag, array);
            let index = translate_expr(b, s, diag, index);
            b.emit(ast::Expr::AccessIndex { place: array, index })
        }
        ExprData::Dot(ref expr, ref ident) => {
            let base = translate_expr(b, s, diag, expr);
            let ty = b.resolve_type(base);
            match b.module.types[ty] {
                ast::TypeDesc::Struct(ref struct_type) => {
                    // field access
                    let field_index = struct_type
                        .fields
                        .iter()
                        .position(|field| field.name.as_str() == ident.as_str());
                    if let Some(field_index) = field_index {
                        b.emit(ast::Expr::AccessField {
                            place: base,
                            index: field_index as u32,
                        })
                    } else {
                        b.error()
                    }
                }
                ast::TypeDesc::Vector { .. } => {
                    // TODO: swizzle place
                    todo!("swizzle")
                }
                _ => b.error(),
            }
        }
        _ => {
            diag.error("expression is not a place")
                .primary_label(span, "")
                .note("expected a variable identifier, array index expression, or field access expression")
                .emit();
            b.error()
        }
    }
}

// what's easier for GLSL output?
// * expressions have no side effects
// * all side effects are in statements

fn translate_expr(
    b: &mut ast::FunctionBuilder,
    s: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    expr: &Expr,
) -> Id<ast::Expr> {
    let span = expr.span;
    match expr.content {
        ExprData::Variable(ref var) => {
            let name = var.as_str();
            match resolve_name(name, b.module, s) {
                None => {
                    diag.error(format!("unresolved name `{name}`"))
                        .primary_label(span, "")
                        .emit();
                    b.error()
                }
                Some(LocalNameResolution::Local(var)) => b.load(var),
                Some(LocalNameResolution::Global(ast::NameResolution::Constant(_))) => {
                    todo!("constants")
                }
                Some(LocalNameResolution::Argument(index)) => {
                    let id = b.argument(index);
                    let ty = b.resolve_type(id);
                    // auto-deref the argument if it's a pointer (reference) type
                    if b.module.pointee_type(ty).is_some() {
                        b.load(id)
                    } else {
                        id
                    }
                }
                Some(LocalNameResolution::Global(ast::NameResolution::GlobalVariable(variable))) => {
                    let expr = b.global(variable);
                    // TODO this assumes that all globals are pointers, is that the case?
                    b.load(expr)
                }
                Some(LocalNameResolution::Global(ast::NameResolution::Function(_))) => {
                    diag.bug("unexpected function expression")
                        .primary_label(span, "")
                        .emit();
                    b.error()
                }
            }
        }
        ExprData::IntConst(v) => b.i32_const(v),
        ExprData::UIntConst(v) => b.u32_const(v),
        ExprData::BoolConst(v) => b.bool_const(v),
        ExprData::FloatConst(v) => b.f32_const(v),
        ExprData::DoubleConst(v) => b.f64_const(v),
        ExprData::Unary(ref op, ref expr) => match op.content {
            UnaryOpData::Inc => {
                todo!()
            }
            UnaryOpData::Dec => {
                todo!()
            }
            UnaryOpData::Add => todo!(),
            UnaryOpData::Minus => {
                let t_expr = translate_expr(b, s, diag, expr);
                let ty = b.resolve_type(t_expr);
                if b.module.is_float_scalar_or_vector(ty) {
                    b.fneg(t_expr)
                } else if b.module.is_signed_integer_scalar_or_vector(ty) {
                    b.sneg(t_expr)
                } else {
                    let display_type = b.module.display_type(ty);
                    diag.error("invalid type for unary operator `-`")
                        .primary_label(expr.span, format!("this is of type {display_type}"))
                        .note(format!(
                            "expected signed integer or floating-point scalar or vector value"
                        ))
                        .emit();
                    b.error()
                }
            }
            UnaryOpData::Not => todo!(),
            UnaryOpData::Complement => todo!(),
        },
        ExprData::Binary(ref op, ref left, ref right) => {
            let t_left = translate_expr(b, s, diag, left);
            let t_right = translate_expr(b, s, diag, right);
            let left_type = b.resolve_type(t_left);
            let right_type = b.resolve_type(t_right);
            if left_type == b.module.error_type || right_type == b.module.error_type {
                // propagate error types silently
                return b.error();
            }

            match op.content {
                BinaryOpData::Add | BinaryOpData::Sub | BinaryOpData::Mult | BinaryOpData::Div => {
                    if left_type != right_type {
                        let display_left_type = b.module.display_type(left_type);
                        let display_right_type = b.module.display_type(right_type);
                        diag.error("type mismatch")
                            .primary_label(left.span, format!("this is of type {display_left_type}"))
                            .primary_label(right.span, format!("this is of type {display_right_type}"))
                            .emit();
                        return b.error();
                    }
                    if b.module.is_float_scalar_or_vector(left_type) {
                        match op.content {
                            BinaryOpData::Add => b.fadd(t_left, t_right),
                            BinaryOpData::Sub => b.fsub(t_left, t_right),
                            BinaryOpData::Mult => b.fmul(t_left, t_right),
                            BinaryOpData::Div => b.fdiv(t_left, t_right),
                            _ => unreachable!(),
                        }
                    } else if b.module.is_signed_integer_scalar_or_vector(left_type) {
                        match op.content {
                            BinaryOpData::Add => b.iadd(t_left, t_right),
                            BinaryOpData::Sub => b.isub(t_left, t_right),
                            BinaryOpData::Mult => b.imul(t_left, t_right),
                            BinaryOpData::Div => b.idiv(t_left, t_right),
                            _ => unreachable!(),
                        }
                    } else {
                        let display_left_type = b.module.display_type(left_type);
                        let display_right_type = b.module.display_type(right_type);
                        diag.error("invalid types for arithmetic operation")
                            .primary_label(left.span, format!("this is of type {display_left_type}"))
                            .primary_label(right.span, format!("this is of type {display_right_type}"))
                            .emit();
                        b.error()
                    }
                }
                BinaryOpData::Or => b.or(t_left, t_right),
                BinaryOpData::Xor => {
                    todo!()
                }
                BinaryOpData::And => b.and(t_left, t_right),
                BinaryOpData::BitOr => b.bit_or(t_left, t_right),
                BinaryOpData::BitXor => b.bit_xor(t_left, t_right),
                BinaryOpData::BitAnd => b.bit_and(t_left, t_right),
                BinaryOpData::Equal => b.eq(t_left, t_right),
                BinaryOpData::NonEqual => b.ne(t_left, t_right),
                BinaryOpData::Lt => b.lt(t_left, t_right),
                BinaryOpData::Gt => b.gt(t_left, t_right),
                BinaryOpData::Lte => b.le(t_left, t_right),
                BinaryOpData::Gte => b.ge(t_left, t_right),
                BinaryOpData::LShift => b.shl(t_left, t_right),
                BinaryOpData::RShift => b.shr(t_left, t_right),
                BinaryOpData::Mod => b.mod_(t_left, t_right),
            }
        }
        ExprData::Ternary(_, _, _) => {
            todo!()
        }
        ExprData::Assignment(ref place, ref op, ref expr) => {
            let t_place = translate_place(b, s, diag, place);
            let t_expr = translate_expr(b, s, diag, expr);
            let place_type = b.resolve_type(t_place);
            let right_type = b.resolve_type(t_expr);
            if place_type == b.module.error_type || right_type == b.module.error_type {
                return b.error();
            }

            match op.content {
                AssignmentOpData::Mult | AssignmentOpData::Div | AssignmentOpData::Add | AssignmentOpData::Sub => {
                    let place_type = b.resolve_type(t_place);
                    let left_type = if let Some(elem_type) = b.module.pointee_type(place_type) {
                        elem_type
                    } else {
                        b.module.error_type
                    };
                    if left_type != right_type {
                        let display_left_type = b.module.display_type(left_type);
                        let display_right_type = b.module.display_type(right_type);
                        diag.error("type mismatch")
                            .primary_label(place.span, format!("this is of type {display_left_type}"))
                            .primary_label(expr.span, format!("this is of type {display_right_type}"))
                            .emit();
                        return b.error();
                    }
                    if b.module.is_float_scalar_or_vector(left_type) {
                        let val = b.load(t_place);
                        let result = match op.content {
                            AssignmentOpData::Add => b.fadd(val, t_expr),
                            AssignmentOpData::Sub => b.fsub(val, t_expr),
                            AssignmentOpData::Mult => b.fmul(val, t_expr),
                            AssignmentOpData::Div => b.fdiv(val, t_expr),
                            _ => unreachable!(),
                        };
                        b.store(t_place, result);
                        result
                    } else if b.module.is_signed_integer_scalar_or_vector(left_type) {
                        let val = b.load(t_place);
                        let result = match op.content {
                            AssignmentOpData::Add => b.iadd(val, t_expr),
                            AssignmentOpData::Sub => b.isub(val, t_expr),
                            AssignmentOpData::Mult => b.imul(val, t_expr),
                            AssignmentOpData::Div => b.idiv(val, t_expr),
                            _ => unreachable!(),
                        };
                        b.store(t_place, result);
                        result
                    } else {
                        let display_left_type = b.module.display_type(left_type);
                        let display_right_type = b.module.display_type(right_type);
                        diag.error("invalid types for arithmetic operation")
                            .primary_label(place.span, format!("this is of type {display_left_type}"))
                            .primary_label(expr.span, format!("this is of type {display_right_type}"))
                            .emit();
                        b.error()
                    }
                }
                AssignmentOpData::Equal => b.assign(t_place, t_expr),
                AssignmentOpData::Mod => {
                    todo!()
                }
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
            let index = translate_expr(b, s, diag, index);
            let array = translate_expr(b, s, diag, array);
            b.array_index(array, index)
        }
        ExprData::FunCall(ref ident, ref args) => match ident.content {
            FunIdentifierData::TypeSpecifier(ref type_spec) => {
                let ty = type_specifier_to_type_desc(b.module, diag, type_spec, None);
                let mut components = vec![];
                for expr in args {
                    let component = translate_expr(b, s, diag, expr);
                    components.push(component);
                }
                b.construct(ty, &components)
            }
            FunIdentifierData::Expr(ref expr) => {
                match expr.content {
                    ExprData::Variable(ref ident) => {
                        // find function
                        //if let Some(function) = b.module.functions_by_name.get(ident)
                        b.error()
                    }
                    _ => {
                        diag.error("invalid function call expression")
                            .primary_label(expr.span, "")
                            .note("expected a function identifier")
                            .emit();
                        b.error()
                    }
                }
            }
        },
        ExprData::Dot(_, _) => {
            todo!()
        }
        ExprData::PostInc(ref expr) => {
            todo!()
            /*let place = translate_place(expr);
            let place_type = builder.resolve_type(place);
            let left_type = if let Some(elem_type) = builder.module.is_pointer_type(place_type) {
                elem_type
            } else {
                builder.module.error_type
            };

            builder.post_increment(place)*/
        }
        ExprData::PostDec(ref expr) => {
            todo!()
            //let place = translate_place(expr);
            //builder.post_increment(place)
        }
        ExprData::Comma(_, _) => {
            todo!()
        }
    }
}

fn translate_initializer(
    b: &mut ast::FunctionBuilder,
    s: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    initializer: &Initializer,
) -> Id<ast::Expr> {
    match initializer.content {
        InitializerData::Simple(ref expr) => translate_expr(b, s, diag, expr),
        InitializerData::List(_) => {
            todo!()
        }
    }
}

fn translate_init_declarator_list(
    b: &mut ast::FunctionBuilder,
    s: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    init_declarator_list: &InitDeclaratorList,
) {
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

    let ty = if constant {
        type_specifier_to_type_desc(
            &mut b.module,
            diag,
            &head.ty.ty.content,
            head.array_specifier.as_deref(),
        )
    } else {
        let base_ty = type_specifier_to_type_desc(
            &mut b.module,
            diag,
            &head.ty.ty.content,
            head.array_specifier.as_deref(),
        );
        b.module.pointer_type(base_ty)
    };

    if let Some(ref name) = head.name {
        let init = if let Some(ref initializer) = head.initializer {
            Some(translate_initializer(b, s, diag, initializer))
        } else {
            None
        };
        let var = b.emit(ast::Expr::LocalVariable {
            name: Some(name.as_str().into()),
            ty,
            init,
        });
        s.declare(name.0.clone(), var);

        for tail_decl in init_declarator_list.tail.iter() {
            let init = if let Some(ref initializer) = tail_decl.initializer {
                Some(translate_initializer(b, s, diag, initializer))
            } else {
                None
            };
            let var = b.emit(ast::Expr::LocalVariable {
                name: Some(tail_decl.ident.ident.as_str().into()),
                ty,
                init,
            });
            s.declare(tail_decl.ident.ident.0.clone(), var);
        }
    }
}

fn translate_statement(
    b: &mut ast::FunctionBuilder,
    s: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    statement: &Statement,
) {
    match statement.content {
        StatementData::Declaration(ref declaration) => match declaration.content {
            DeclarationData::FunctionPrototype(_) => {
                todo!()
            }
            DeclarationData::InitDeclaratorList(ref decl_list) => {
                translate_init_declarator_list(b, s, diag, decl_list);
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
                translate_expr(b, s, diag, expr);
            }
        }
        StatementData::Selection(ref selection) => {
            let condition = translate_expr(b, s, diag, &selection.cond);
            b.if_(condition);
            match selection.rest.content {
                SelectionRestStatementData::Statement(ref statement) => {
                    translate_statement(b, s, diag, statement);
                }
                SelectionRestStatementData::Else(ref then_branch, ref else_branch) => {
                    translate_statement(b, s, diag, then_branch);
                    b.else_();
                    translate_statement(b, s, diag, else_branch);
                }
            }
            b.end_if();
        }
        StatementData::Switch(_) => {}
        StatementData::CaseLabel(_) => {}
        StatementData::Iteration(ref iteration) => match iteration.content {
            IterationStatementData::While(ref condition, ref body) => {
                b.loop_();
                let condition = match condition.content {
                    ConditionData::Expr(ref expr) => translate_expr(b, s, diag, expr),
                    ConditionData::Assignment(_, _, _) => {
                        todo!()
                    }
                };
                let not_condition = b.not(condition);
                b.if_(not_condition);
                b.break_();
                b.end_if();
                translate_statement(b, s, diag, body);
                b.end_loop();
            }
            IterationStatementData::DoWhile(ref body, ref condition) => {
                b.loop_();
                translate_statement(b, s, diag, body);
                let condition = translate_expr(b, s, diag, condition);
                let not_condition = b.not(condition);
                b.if_(not_condition);
                b.break_();
                b.end_if();
                b.end_loop();
            }
            IterationStatementData::For(ref init, ref rest, ref body) => {
                match init.content {
                    ForInitStatementData::Expression(ref expr) => {
                        if let Some(expr) = expr {
                            translate_expr(b, s, diag, expr);
                        }
                    }
                    ForInitStatementData::Declaration(ref decl) => match decl.content {
                        DeclarationData::InitDeclaratorList(ref init_decl_list) => {
                            translate_init_declarator_list(b, s, diag, init_decl_list);
                        }
                        _ => {
                            panic!("invalid declaration")
                        }
                    },
                };

                b.loop_();
                if let Some(ref condition) = rest.condition {
                    let condition = match condition.content {
                        ConditionData::Expr(ref expr) => translate_expr(b, s, diag, expr),
                        ConditionData::Assignment(_, _, _) => {
                            todo!()
                        }
                    };
                    let not_condition = b.not(condition);
                    b.if_(not_condition);
                    b.break_();
                    b.end_if();
                }
                translate_statement(b, s, diag, body);
                if let Some(ref post_expr) = rest.post_expr {
                    translate_expr(b, s, diag, post_expr);
                }
                b.end_loop();
            }
        },
        StatementData::Jump(ref jump) => match jump.content {
            JumpStatementData::Continue => {
                b.continue_();
            }
            JumpStatementData::Break => {
                b.break_();
            }
            JumpStatementData::Return(ref result) => {
                let value = if let Some(ref result) = result {
                    let value = translate_expr(b, s, diag, result);
                    Some(value)
                } else {
                    None
                };
                b.return_(value);
            }
            JumpStatementData::Discard => {
                b.discard();
            }
        },
        StatementData::Compound(ref compound_statement) => translate_compound_statement(b, s, diag, compound_statement),
    }
}

fn translate_compound_statement(
    builder: &mut ast::FunctionBuilder,
    scope: &mut ScopeStack,
    diag: &mut DiagnosticSink,
    compound_statement: &CompoundStatement,
) {
    for stmt in compound_statement.statement_list.iter() {
        translate_statement(builder, scope, diag, stmt);
    }
}

fn translate_function_definition(
    m: &mut ast::Module,
    diag: &mut DiagnosticSink,
    function_definition: &FunctionDefinition,
) {
    let name = function_definition.prototype.name.as_str();

    // build function type
    let mut arguments = Vec::new();
    let mut arg_names = HashMap::new();
    let return_type = m.void_type;

    for param in function_definition.prototype.parameters.iter() {
        match param.content {
            FunctionParameterDeclarationData::Named(ref type_qualifier, ref declarator) => {
                let mut input = false;
                let mut output = false;
                if let Some(qual) = type_qualifier {
                    for qual in qual.qualifiers.iter() {
                        match qual.content {
                            TypeQualifierSpecData::Storage(ref storage_qualifier) => match storage_qualifier.content {
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
                            },
                            _ => {
                                panic!("unexpected type qualifier")
                            }
                        }
                    }
                }
                let ty = type_specifier_to_type_desc(
                    m,
                    diag,
                    &declarator.ty.content,
                    declarator.ident.array_spec.as_deref(),
                );
                arguments.push(ty);

                let index = (arguments.len() - 1) as u32;
                if arg_names.insert(SmolStr::from(name), index).is_some() {
                    diag.error("duplicate argument name")
                        .primary_label(
                            declarator.ident.ident.span,
                            "an argument with the same name already exists",
                        )
                        .emit();
                }
            }
            FunctionParameterDeclarationData::Unnamed(_, _) => {
                todo!("unnamed function parameters")
            }
        }
    }

    let function_type = TypeDesc::Function { return_type, arguments };
    let function_type = m.types.add(function_type);

    let mut function_builder = m.build_function(Some(name.into()), function_type);
    let mut scope_stack = ScopeStack::new(arg_names);
    translate_compound_statement(
        &mut function_builder,
        &mut scope_stack,
        diag,
        &function_definition.statement,
    );
    match function_builder.finish() {
        Ok(_) => {}
        Err(ast::Error::NameConflict) => diag
            .error(format!("a function with the name `{name}` has already been defined"))
            .primary_label(function_definition.prototype.name.span, "")
            .emit(),
    }
}

fn translate_translation_unit(m: &mut ast::Module, diag: &mut DiagnosticSink, translation_unit: &TranslationUnit) {
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

                        // helper function to create a global var
                        let do_insert_global =
                            |m: &mut Module, diag: &mut DiagnosticSink, name: &Identifier, ty| match m
                                .insert_global_variable(Some(name.0.clone()), ty)
                            {
                                Ok(_) => {}
                                Err(ast::Error::NameConflict) => diag
                                    .error(format!(
                                        "a global variable with the name `{}` has already been defined",
                                        name.as_str()
                                    ))
                                    .primary_label(name.span, "")
                                    .emit(),
                            };

                        // call type_specifier_to_type_desc even if there's no variable declared, because we might be declaring a user defined type at the same time
                        let ty =
                            type_specifier_to_type_desc(m, diag, &decl.ty.ty.content, decl.array_specifier.as_deref());
                        let pointer_ty = m.pointer_type(ty);
                        let is_interface = uniform || input || output;

                        if let Some(ref name) = decl.name {
                            do_insert_global(m, diag, name, pointer_ty);

                            for tail_decl in declarator_list.tail.iter() {
                                let ty = type_specifier_to_type_desc(
                                    m,
                                    diag,
                                    &decl.ty.ty.content,
                                    tail_decl.ident.array_spec.as_deref(),
                                );
                                let pointer_ty = m.pointer_type(ty);
                                do_insert_global(m, diag, name, pointer_ty);
                            }
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
                translate_function_definition(m, diag, def);
                /*if def.prototype.name.as_str() == "main" {
                } else {
                }*/
            }
            _ => {}
        }
    }
}

pub fn translate_glsl(
    module: &mut ast::Module,
    diag_writer: &mut dyn WriteColor,
    aux_sources: &SourceFiles,
    source: &str,
    source_id: &str,
) -> Result<(), ()> {
    let mut pp = Preprocessor::new_with_fs(&aux_sources);
    let main_source = SourceFile::new(source_id.into(), source.to_string());

    // setup preprocessor and construct the lexer input
    let input_file = pp.open_source(&main_source.text, "").with_state(
        ProcessorState::builder()
            .extension(ext_name!("GL_GOOGLE_include_directive"), ExtensionBehavior::Enable)
            .finish(),
    );

    // parse the GLSL into a translation unit
    let mut translation_unit = TranslationUnit::parse_with_options::<Lexer<&SourceFiles>>(
        input_file,
        &ParseOptions {
            target_vulkan: true,
            ..Default::default()
        },
    )
    .unwrap()
    .0;

    let diag_config = term::Config::default();
    let mut diag_sink = DiagnosticSink::new(diag_writer, diag_config, &aux_sources, &main_source, &pp);
    translate_translation_unit(module, &mut diag_sink, &translation_unit);
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::{
        ast,
        glsl::{translate_glsl, DiagnosticSink, Preprocessor, SourceFiles},
    };
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
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

        vec4 f(vec4 sdf) {
            if (position) {
                return vec4(4.0, 0.0, 0.0, 1.0);
            } else {
                return vec4(vec2(1.0, 0.0), mat4(1.0));
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
        let mut sources = SourceFiles::new();
        sources.register_source("common.glsl", GLSL_COMMON);
        sources.register_source("stroke.glsl", STROKE_GLSL);

        let mut module = ast::Module::new();
        let mut diag_writer = StandardStream::stderr(ColorChoice::Always);
        translate_glsl(&mut module, &mut diag_writer, &sources, GLSL_SOURCE_1, "source_1.glsl").unwrap();
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
