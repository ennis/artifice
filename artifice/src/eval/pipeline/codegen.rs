use crate::{
    eval::pipeline::{Program, TypeDesc},
    model::typedesc::{ImageDimension, PrimitiveType},
};
use glsl_lang::{
    ast,
    ast::{
        ArraySpecifier, ArraySpecifierDimension, ArrayedIdentifier, AssignmentOp, BinaryOp, Block, CaseLabel,
        CompoundStatement, Condition, Declaration, Expr, ExprStatement, ExternalDeclaration, ForInitStatement,
        ForRestStatement, FullySpecifiedType, FunIdentifier, FunctionDefinition, FunctionParameterDeclaration,
        FunctionParameterDeclarator, FunctionPrototype, Identifier, InitDeclaratorList, Initializer,
        InterpolationQualifier, IterationStatement, JumpStatement, LayoutQualifier, LayoutQualifierSpec, NodeContent,
        NodeDisplay, NodeSpan, PrecisionQualifier, Preprocessor, PreprocessorDefine, PreprocessorElseIf,
        PreprocessorError, PreprocessorExtension, PreprocessorExtensionBehavior, PreprocessorExtensionName,
        PreprocessorIf, PreprocessorIfDef, PreprocessorIfNDef, PreprocessorInclude, PreprocessorLine,
        PreprocessorPragma, PreprocessorUndef, PreprocessorVersion, PreprocessorVersionProfile, SelectionRestStatement,
        SelectionStatement, SingleDeclaration, SingleDeclarationNoType, Statement, StorageQualifier,
        StructFieldSpecifier, StructSpecifier, SwitchStatement, TranslationUnit, TypeName, TypeQualifier,
        TypeQualifierSpec, TypeSpecifier, TypeSpecifierNonArray, UnaryOp,
    },
    visitor::{Visit, VisitorMut},
};
use kyute_common::Atom;
use rusqlite::types::Type;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    ptr::write,
};

/// Sized uniform.
struct Uniform {
    ty: TypeDesc,
    name: Atom,
}

/// Opaque uniform types (textures, samplers, buffers).
struct OpaqueUniform {
    ty: TypeDesc,
    name: Atom,
    set: u32,
    binding: u32,
}

struct Buffer {
    ty: TypeDesc,
    name: Atom,
}

struct InputOutput {
    location: u32,
    ty: TypeDesc,
    name: Atom,
}

/// Texture descriptor.
struct Texture {}

///
/// # Examples:
///
/// `vec3[]` => FV3R
/// `vec3[4]` => FV3A4
/// `struct Stroke[]` => S6StrokeR
/// `mat4x3` => M43
///
fn generate_mangled_type_name(out: &mut dyn fmt::Write, ty: &TypeDesc) -> fmt::Result {
    match ty {
        TypeDesc::Void => write!(out, "Z")?,
        TypeDesc::Primitive(PrimitiveType::Float) => write!(out, "F")?,
        TypeDesc::Primitive(PrimitiveType::Int) => write!(out, "I")?,
        TypeDesc::Primitive(PrimitiveType::UnsignedInt) => write!(out, "U")?,
        TypeDesc::Primitive(PrimitiveType::Bool) => write!(out, "B")?,
        TypeDesc::Primitive(PrimitiveType::Double) => write!(out, "D")?,
        TypeDesc::Vector { elem_ty, len } => {
            generate_mangled_type_name(out, &TypeDesc::Primitive(*elem_ty))?;
            write!(out, "V{}", len)?;
        }
        TypeDesc::Matrix { elem_ty, rows, columns } => {
            generate_mangled_type_name(out, &TypeDesc::Primitive(*elem_ty))?;
            write!(out, "M{}{}", rows, columns)?;
        }
        TypeDesc::Array { elem_ty, len } => {
            generate_mangled_type_name(out, elem_ty)?;
            write!(out, "A{}", len)?;
        }
        TypeDesc::RuntimeArray(elem_ty) => {
            generate_mangled_type_name(out, elem_ty)?;
            write!(out, "R")?;
        }
        TypeDesc::Struct(s) => {
            write!(out, "T{}{}", s.name.len(), s.name)?;
        }
        TypeDesc::SampledImage(image) => {
            match image.ms {
                true => write!(out, "Gm")?,
                false => write!(out, "Gs")?,
            };
            generate_mangled_type_name(out, &TypeDesc::Primitive(image.sampled_ty))?;
            match image.dim {
                ImageDimension::Dim1D => write!(out, "1")?,
                ImageDimension::Dim2D => write!(out, "2")?,
                ImageDimension::Dim3D => write!(out, "3")?,
                ImageDimension::DimCube => write!(out, "C")?,
                ImageDimension::Dim1DArray => write!(out, "A")?,
                ImageDimension::Dim2DArray => write!(out, "B")?,
            }
        }
        TypeDesc::Image(image) => {
            match image.ms {
                true => write!(out, "Hm")?,
                false => write!(out, "Hs")?,
            };
            generate_mangled_type_name(out, &TypeDesc::Primitive(image.element_ty))?;
            match image.dim {
                ImageDimension::Dim1D => write!(out, "1")?,
                ImageDimension::Dim2D => write!(out, "2")?,
                ImageDimension::Dim3D => write!(out, "3")?,
                ImageDimension::DimCube => write!(out, "C")?,
                ImageDimension::Dim1DArray => write!(out, "A")?,
                ImageDimension::Dim2DArray => write!(out, "B")?,
            }
        }
        TypeDesc::Pointer(ty) => {
            generate_mangled_type_name(out, ty);
            write!(out, "P")?
        }
        TypeDesc::String => write!(out, "Y")?,
        TypeDesc::Sampler => write!(out, "S")?,
        TypeDesc::ShadowSampler => write!(out, "W")?,
        TypeDesc::Unknown => write!(out, "?")?,
    };

    Ok(())
}

/// GLSL shader code generation context.
pub struct CodegenContext {
    glsl_version: u32,
    sized_uniforms: Vec<Uniform>,
    opaque_uniforms: Vec<OpaqueUniform>,
    buffers: Vec<Buffer>,
    inputs: Vec<InputOutput>,
    outputs: Vec<InputOutput>,
    names: HashSet<Atom>,
    mangled_type_names: HashMap<TypeDesc, Atom>,
}

impl CodegenContext {
    /// Creates a new code generation context.
    pub fn new(glsl_version: u32) -> CodegenContext {
        CodegenContext {
            glsl_version,
            sized_uniforms: vec![],
            opaque_uniforms: vec![],
            buffers: vec![],
            inputs: vec![],
            outputs: vec![],
            names: Default::default(),
            mangled_type_names: Default::default(),
        }
    }

    /// Returns the generated name for a buffer interface block containing a value of the given type.
    ///
    /// We don't require the user to declare an interface block when declaring a buffer-backed parameter.
    /// This means that we have to turn:
    ///
    /// ```glsl
    /// buffer float[] data1;
    /// buffer float[] data2;
    /// ```
    ///
    /// into one buffer interface block per type:
    ///
    /// ```glsl
    /// layout(buffer_reference) buffer float__array;
    /// layout(buffer_reference, std430) buffer float__array { float[] contents; };
    /// ```
    ///
    /// The two buffers share the same type, so they have the same underlying block.
    /// However, in the source code, we must replace all references to `data1` and `data2` with `data1.contents` and `data2.contents`.
    fn mangle_type(&mut self, ty: &TypeDesc) -> Atom {
        if let Some(name) = self.mangled_type_names.get(ty) {
            return name.clone();
        }
        let mut s = String::new();
        generate_mangled_type_name(&mut s, ty).unwrap();
        let name = self.make_unique_name(s);
        self.mangled_type_names.insert(ty.clone(), name.clone());
        name
    }

    /// Declares the following name to be in use by the shader.
    ///
    /// If the name is already in use, will return a unique name.
    pub fn make_unique_name(&mut self, name: impl Into<Atom>) -> Atom {
        // TODO
        let orig_name = name.into();
        let mut unique_name = orig_name.clone();
        while self.names.contains(&unique_name) {
            unique_name = format!("{}_{}", orig_name, self.names.len()).into();
        }
        self.names.insert(unique_name.clone());
        unique_name
    }

    /*/// Adds an opaque uniform (texture or samplers).
        pub fn add_opaque_uniform(&mut self, name: impl Into<Atom>, ty: TypeDesc) -> (u32, u32) {}
    */

    /// Adds a shader input (like `in vec3 position`).
    ///
    /// Returns the assigned location for the input (the `n` in `layout(location=n)`).
    pub fn add_input(&mut self, name: impl Into<Atom>, ty: TypeDesc) -> u32 {
        let location = self.inputs.len() as u32;
        let name = name.into();
        self.inputs.push(InputOutput { name, ty, location });
        location
    }

    /// Adds a shader output (like `out vec4 color`).
    pub fn add_output(&mut self, name: impl Into<Atom>, ty: TypeDesc, location: u32) {
        assert!(
            self.outputs.iter().find(|output| output.location == location).is_none(),
            "the given location was already assigned to another output"
        );
        let name = name.into();
        self.outputs.push(InputOutput { name, ty, location });
    }

    /// Adds a buffer uniform (like `layout(buffer_reference) buffer MyBuffer_tag { float[] contents; };`).
    pub fn add_buffer(&mut self, name: impl Into<Atom>, ty: TypeDesc) {
        self.buffers.push(Buffer { name: name.into(), ty });
    }

    /*/// Returns the writer for the declaration part of the shader.
    pub fn declaration_writer(&mut self) -> impl fmt::Write + '_ {
        todo!()
    }

    /// Returns the writer for the statements in the main function.
    pub fn main_function_writer(&mut self) -> impl fmt::Write + '_ {
        todo!()
    }*/

    /// Generates the source code of the shader.
    pub fn generate(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        let mut current_set = 0;
        let mut current_binding = 0;

        // --- Uniforms ---
        // TODO multiple uniform buffers, per variability
        // TODO extract the layout of the uniform block
        if !self.sized_uniforms.is_empty() {
            writeln!(
                out,
                "layout(set = {}, binding = {}, std140) uniform TimeVaryings {{",
                current_set, current_binding
            )?;
            // write variables
            for uniform in &self.sized_uniforms {
                writeln!(out, "    {} {};", uniform.ty.display_glsl(), uniform.name)?;
            }

            // also write pointers to buffers

            writeln!(out, "}};")?;
        }
        current_binding += 1;

        // --- opaque uniforms ---
        // Opaque uniforms each get their own binding number
        for opaque_uniform in &self.opaque_uniforms {
            writeln!(
                out,
                "layout(set = {}, binding = {}) uniform {} {};",
                current_set,
                current_binding,
                opaque_uniform.ty.display_glsl(),
                opaque_uniform.name,
            )?;
            current_binding += 1;
        }

        // --- I/O interface ---
        for input in self.inputs.iter() {
            writeln!(
                out,
                "layout(location={}) in {} {};",
                input.location,
                input.ty.display_glsl(),
                input.name
            )?;
        }

        for output in self.outputs.iter() {
            writeln!(
                out,
                "layout(location={}) out {} {};",
                output.location,
                output.ty.display_glsl(),
                output.name
            )?;
        }

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Program codegen
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Our GLSL AST rewriter.
///
/// It does the following:
/// - adds a file ID suffix to all names (except locals and function parameters).
/// - rewrites usages of buffer_references
///   
struct AppendFileIdRewriter<'a> {
    cg: &'a mut CodegenContext,
    /// First declaration sites of all symbols.
    non_type_declarations: HashMap<String, String>,
    type_declarations: HashMap<String, String>,
    shadowed_names: HashSet<String>,
}

impl<'a> AppendFileIdRewriter<'a> {
    fn new(cg: &'a mut CodegenContext) -> AppendFileIdRewriter<'a> {
        AppendFileIdRewriter {
            cg,
            non_type_declarations: Default::default(),
            type_declarations: Default::default(),
            shadowed_names: Default::default(),
        }
    }
}

impl<'a> glsl_lang::visitor::VisitorMut for AppendFileIdRewriter<'a> {
    fn visit_type_name(&mut self, type_name: &mut ast::TypeName) -> Visit {
        if let Some(udt) = self.type_declarations.get(type_name.as_str()) {
            type_name.content.0 = udt.into();
        }
        Visit::Parent
    }

    fn visit_function_definition(&mut self, def: &mut ast::FunctionDefinition) -> Visit {
        self.shadowed_names.clear();
        for param in def.prototype.parameters.iter() {
            if let ast::FunctionParameterDeclarationData::Named(_, ref param) = param.content {
                self.shadowed_names.insert(param.ident.ident.to_string());
            }
        }
        Visit::Children
    }

    fn visit_function_prototype(&mut self, fp: &mut ast::FunctionPrototype) -> Visit {
        let new_name = format!("{}__{}", fp.name.as_str(), fp.span.unwrap().source_id().number());
        self.non_type_declarations.insert(fp.name.to_string(), new_name.clone());
        fp.name.content.0 = new_name.into();
        Visit::Parent
    }

    fn visit_single_declaration(&mut self, decl: &mut ast::SingleDeclaration) -> Visit {
        if let Some(ref mut name) = decl.name {
            let new_name = format!("{}__{}", name.as_str(), name.span.unwrap().source_id().number());
            self.non_type_declarations.insert(name.to_string(), new_name.clone());
            name.0 = new_name.into();
        }
        Visit::Children
    }

    fn visit_single_declaration_no_type(&mut self, decl: &mut ast::SingleDeclarationNoType) -> Visit {
        let new_name = format!(
            "{}_{}",
            decl.ident.ident.as_str(),
            decl.span.unwrap().source_id().number()
        );
        self.non_type_declarations
            .insert(decl.ident.ident.to_string(), new_name.clone());
        decl.ident.ident.0 = new_name.into();
        Visit::Children
    }

    fn visit_struct_specifier(&mut self, s: &mut StructSpecifier) -> Visit {
        if let Some(ref mut name) = s.name {
            let new_name = format!("{}__{}", name.as_str(), name.span.unwrap().source_id().number());
            self.type_declarations.insert(name.to_string(), new_name.clone());
            name.content.0 = new_name.into();
        }
        Visit::Children
    }

    fn visit_expr(&mut self, expr: &mut ast::Expr) -> Visit {
        match expr.content {
            ast::ExprData::Variable(ref mut ident) => {
                if !self.shadowed_names.contains(ident.as_str()) {
                    if let Some(ident2) = self.non_type_declarations.get(ident.as_str()) {
                        ident.content.0 = ident2.into();
                    }
                }
                Visit::Parent
            }
            _ => Visit::Children,
        }
    }
}

/// AST rewriter adds another indirection to `buffer_reference` variables:
/// ```glsl
/// // Before:
/// buffer float[] data;
/// void main() {
///     ...
///     data.x
///     ...
/// }
/// // After:
/// void main() {
///     ...
///     data.__d.x
///     ...
/// }
/// ```
struct BufferReferenceRewriter {
    pointer_vars: HashSet<String>,
}

impl VisitorMut for BufferReferenceRewriter {
    fn visit_expr(&mut self, expr: &mut Expr) -> Visit {
        match expr.content {
            ast::ExprData::Variable(ref mut ident) => {
                if self.pointer_vars.contains(ident.as_str()) {
                    expr.content = ast::ExprData::Dot(Box::new(expr.clone()), "__d".into_node());
                }
                Visit::Parent
            }
            _ => Visit::Children,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        eval::pipeline::{
            codegen::{AppendFileIdRewriter, CodegenContext},
            program::{Preprocessor, Vfs},
            Program, TypeDesc,
        },
        model::typedesc::{ImageDimension, PrimitiveType, SampledImageType, StructType},
    };
    use glsl_lang::{
        transpiler::glsl::{show_translation_unit, FormattingState},
        visitor::{HostMut, VisitorMut},
    };
    use std::sync::Arc;

    // language=glsl
    const COMMON: &str = r#"
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

    struct Bar {
        Foo foo;
        int[4] values;
    };
    "#;

    // language=glsl
    const SOURCE_A: &str = r#"
        #include "common.glsl" 

        in vec3 position;               
        in vec3 normals;                
        uniform in vec3 param;          
        uniform in vec3 param2[45];     
        out vec4 normals = f(position); 

        // opaque types
        uniform in sampler tex1;
        uniform in image2DMS tex2;
        uniform sampler sampler1;
        uniform texture2D tex3;

        // buffers
        buffer float[] floatArrayPointer;
        struct MyStruct { Foo b; }; 
        buffer MyStruct[] myStructArrayPointer;

        vec4 f() {
            return vec4(0.0,0.0,0.0,1.0);
        }
    "#;

    // language=glsl
    const INLINE_EXPR: &str = r#"
    in vec3 color;
    out vec3 o_color = color * vec3(1.0, 2.0, 3.0);
    "#;

    #[test]
    fn type_mangling() {
        let mut cg = CodegenContext::new(460);

        let mangled = cg.mangle_type(&TypeDesc::RuntimeArray(Arc::new(TypeDesc::Primitive(
            PrimitiveType::Int,
        ))));
        eprintln!("{}", mangled);

        let mangled = cg.mangle_type(&TypeDesc::RuntimeArray(Arc::new(TypeDesc::Vector {
            len: 3,
            elem_ty: PrimitiveType::Int,
        })));
        eprintln!("{}", mangled);

        let mangled = cg.mangle_type(&TypeDesc::RuntimeArray(Arc::new(TypeDesc::Array {
            len: 32,
            elem_ty: Arc::new(TypeDesc::Struct(Arc::new(StructType {
                name: "Stroke".into(),
                fields: vec![],
            }))),
        })));
        eprintln!("{}", mangled);

        let mangled = cg.mangle_type(&TypeDesc::Array {
            len: 16,
            elem_ty: Arc::new(TypeDesc::Matrix {
                elem_ty: PrimitiveType::Float,
                rows: 4,
                columns: 4,
            }),
        });
        eprintln!("{}", mangled);

        let mangled = cg.mangle_type(&TypeDesc::RuntimeArray(Arc::new(TypeDesc::SampledImage(Arc::new(
            SampledImageType {
                sampled_ty: PrimitiveType::Float,
                dim: ImageDimension::Dim2D,
                ms: false,
            },
        )))));
        eprintln!("{}", mangled);

        let mangled = cg.mangle_type(&TypeDesc::RuntimeArray(Arc::new(TypeDesc::Sampler)));
        eprintln!("{}", mangled);
    }

    #[test]
    fn rewriters() {
        let mut vfs = Vfs::new();
        vfs.register_source("common.glsl", COMMON);
        vfs.dump();
        let mut pp = Preprocessor::new_with_fs(vfs);
        let mut program = Program::new(SOURCE_A, "src_a", &mut pp).unwrap();

        let mut cg = CodegenContext::new(460);
        let mut file_id_rewriter = AppendFileIdRewriter::new(&mut cg);
        program.translation_unit.visit_mut(&mut file_id_rewriter);

        let mut out = String::new();
        let formatting_state = FormattingState::default();
        show_translation_unit(&mut out, &program.translation_unit, formatting_state).unwrap();
        eprintln!("{}", out);
    }

    #[test]
    fn test_codegen() {
        let mut ctx = CodegenContext::new(460);
    }
}
