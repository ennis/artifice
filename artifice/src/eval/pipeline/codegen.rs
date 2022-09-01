use crate::{
    eval::{
        pipeline::{Binding, CodegenResult, Program, ShaderResourceIndex, SsaName, TypeDesc, Variable},
        Variability,
    },
    model::typedesc::{ImageDimension, PrimitiveType},
};
use glsl_lang::{
    ast,
    ast::{
        ArraySpecifier, ArraySpecifierDimension, ArrayedIdentifier, AssignmentOp, BinaryOp, Block, CaseLabel,
        CompoundStatement, Condition, Declaration, DeclarationData, Expr, ExprStatement, ExternalDeclaration,
        ForInitStatement, ForRestStatement, FullySpecifiedType, FunIdentifier, FunctionDefinition,
        FunctionParameterDeclaration, FunctionParameterDeclarator, FunctionPrototype, Identifier, InitDeclaratorList,
        Initializer, InterpolationQualifier, IterationStatement, JumpStatement, LayoutQualifier, LayoutQualifierSpec,
        NodeContent, NodeDisplay, NodeSpan, PrecisionQualifier, Preprocessor, PreprocessorDefine, PreprocessorElseIf,
        PreprocessorError, PreprocessorExtension, PreprocessorExtensionBehavior, PreprocessorExtensionName,
        PreprocessorIf, PreprocessorIfDef, PreprocessorIfNDef, PreprocessorInclude, PreprocessorLine,
        PreprocessorPragma, PreprocessorUndef, PreprocessorVersion, PreprocessorVersionProfile, SelectionRestStatement,
        SelectionStatement, SingleDeclaration, SingleDeclarationNoType, Statement, StorageQualifier,
        StructFieldSpecifier, StructSpecifier, SwitchStatement, TranslationUnit, TypeName, TypeQualifier,
        TypeQualifierSpec, TypeSpecifier, TypeSpecifierNonArray, UnaryOp,
    },
    transpiler::glsl::{
        show_block, show_function_definition, show_function_prototype, show_identifier, show_init_declarator_list,
        show_initializer, show_preprocessor, show_translation_unit, FormattingState,
    },
    visitor::{HostMut, Visit, VisitorMut},
};
use kyute_common::Atom;
use std::{
    collections::{HashMap, HashSet},
    fmt,
    fmt::Write,
    sync::Arc,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Program with name substitutions.
pub(crate) struct BoundProgram<'a> {
    pub(crate) program: &'a Program,
    pub(crate) substitutions: HashMap<Arc<str>, Arc<str>>,
}

pub(crate) enum GlslVariable {
    UniformBlock {
        set: u32,
        binding: u32,
        block_name: Arc<str>,
        fields: Vec<(Arc<str>, TypeDesc)>,
    },
    Uniform {
        ty: TypeDesc,
        name: Arc<str>,
        set: u32,
        binding: u32,
    },
    Input {
        location: u32,
        ty: TypeDesc,
        name: Arc<str>,
    },
    Output {
        location: u32,
        ty: TypeDesc,
        name: Arc<str>,
    },
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

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////

/// GLSL AST rewriter.
///
/// It does the following:
/// - adds a file ID suffix to all names (except locals and function parameters).
/// - rewrites usages of buffer_references
/// - replaces
struct NameSubstitutionRewriter<'a> {
    cg: &'a mut CodegenContext,
    non_type_name_substitutions: HashMap<Arc<str>, Arc<str>>,
    type_name_substitutions: HashMap<Arc<str>, Arc<str>>,
    shadowed_names: HashSet<String>,
}

impl<'a> VisitorMut for NameSubstitutionRewriter<'a> {
    fn visit_type_name(&mut self, type_name: &mut ast::TypeName) -> Visit {
        if let Some(udt) = self.type_name_substitutions.get(type_name.as_str()) {
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
        // rewrite `void function() { ... }` to `void function__<source_id>() { ... }`
        let new_name = format!("{}__{}", fp.name.as_str(), fp.span.unwrap().source_id().number());
        self.non_type_name_substitutions
            .insert(fp.name.to_string().into(), new_name.clone().into());
        fp.name.content.0 = new_name.into();
        Visit::Parent
    }

    fn visit_single_declaration(&mut self, decl: &mut ast::SingleDeclaration) -> Visit {
        if let Some(ref mut name) = decl.name {
            // don't rename if there's already a substitution for the name
            if !self.non_type_name_substitutions.contains_key(name.as_str()) {
                let new_name = format!("{}__{}", name.as_str(), name.span.unwrap().source_id().number());
                self.non_type_name_substitutions
                    .insert(name.to_string().into(), new_name.clone().into());
                name.0 = new_name.into();
            }
        }
        Visit::Children
    }

    fn visit_single_declaration_no_type(&mut self, decl: &mut ast::SingleDeclarationNoType) -> Visit {
        if !self.non_type_name_substitutions.contains_key(decl.ident.ident.as_str()) {
            let new_name = format!(
                "{}__{}",
                decl.ident.ident.as_str(),
                decl.span.unwrap().source_id().number()
            );
            self.non_type_name_substitutions
                .insert(decl.ident.ident.to_string().into(), new_name.clone().into());
            decl.ident.ident.0 = new_name.into();
        }
        Visit::Children
    }

    fn visit_struct_specifier(&mut self, s: &mut StructSpecifier) -> Visit {
        if let Some(ref mut name) = s.name {
            let new_name = format!("{}__{}", name.as_str(), name.span.unwrap().source_id().number());
            self.type_name_substitutions
                .insert(name.to_string().into(), new_name.clone().into());
            name.content.0 = new_name.into();
        }
        Visit::Children
    }

    fn visit_expr(&mut self, expr: &mut ast::Expr) -> Visit {
        match expr.content {
            ast::ExprData::Variable(ref mut ident) => {
                if !self.shadowed_names.contains(ident.as_str()) {
                    if let Some(ident2) = self.non_type_name_substitutions.get(ident.as_str()) {
                        ident.content.0 = ident2.into();
                    }
                }
                Visit::Parent
            }
            _ => Visit::Children,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Codegen
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Maximum supported number of descriptor sets.
const MAX_SETS: usize = 8;

/// GLSL shader code generation context.
pub(crate) struct CodegenContext {
    pub(crate) names: HashSet<Atom>,
    pub(crate) mangled_type_names: HashMap<TypeDesc, Atom>,
    pub(crate) declarations: String,
    pub(crate) function_definitions: String,
    pub(crate) body: String,
}

fn is_output_interface(decl: &ast::SingleDeclaration) -> bool {
    if let Some(ref qualifiers) = decl.ty.qualifier {
        qualifiers.qualifiers.iter().any(|qual| match qual.content {
            ast::TypeQualifierSpecData::Storage(ref storage_qual) => {
                storage_qual.content == ast::StorageQualifierData::Out
            }
            _ => false,
        })
    } else {
        false
    }
}

fn is_input_interface(decl: &ast::SingleDeclaration) -> bool {
    if let Some(ref qualifiers) = decl.ty.qualifier {
        qualifiers.qualifiers.iter().any(|qual| match qual.content {
            ast::TypeQualifierSpecData::Storage(ref storage_qual) => {
                storage_qual.content == ast::StorageQualifierData::In
                    || storage_qual.content == ast::StorageQualifierData::Uniform
            }
            _ => false,
        })
    } else {
        false
    }
}

impl CodegenContext {
    /// Creates a new code generation context.
    pub(crate) fn new() -> CodegenContext {
        CodegenContext {
            names: Default::default(),
            mangled_type_names: Default::default(),
            declarations: "".to_string(),
            function_definitions: "".to_string(),
            body: "".to_string(),
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

    /// Adds a program.
    pub(crate) fn write_program(&mut self, program: &Program, interface_bindings: &[Binding]) {
        let mut formatting_state = FormattingState::default();

        let mut non_type_name_substitutions = HashMap::new();

        for (b, i) in interface_bindings.iter().zip(program.interface.iter()) {
            match b {
                Binding::Default => {}
                Binding::Variable { name, ssa_index } => {
                    non_type_name_substitutions.insert(i.name.clone(), format!("{name}_{ssa_index}").into());
                }
                Binding::Uniform { name } => {
                    non_type_name_substitutions.insert(i.name.clone(), name.clone());
                }
            }
        }

        let substituted_program = {
            let mut rewriter = NameSubstitutionRewriter {
                cg: self,
                non_type_name_substitutions,
                type_name_substitutions: HashMap::new(),
                shadowed_names: Default::default(),
            };
            // a bit expensive, but I don't want to write my own transpiler
            let mut p = program.translation_unit.clone();
            p.visit_mut(&mut rewriter);
            p
        };

        let write_output_interface = |s: &mut String,
                                      formatting_state: &mut FormattingState,
                                      ty: &ast::FullySpecifiedType,
                                      name: Option<&ast::Identifier>,
                                      initializer: Option<&ast::Initializer>| {
            if let Some(name) = name {
                if let Some(initializer) = initializer {
                    write!(s, "    ").unwrap();
                    show_identifier(s, name, formatting_state).unwrap();
                    write!(s, " = ").unwrap();
                    show_initializer(s, initializer, formatting_state).unwrap();
                    writeln!(s, ";").unwrap();
                }
            }
        };

        for decl in substituted_program.0.iter() {
            match decl.content {
                ast::ExternalDeclarationData::Declaration(ref decl) => match decl.content {
                    ast::DeclarationData::InitDeclaratorList(ref declarator_list) => {
                        if is_output_interface(&declarator_list.head) {
                            write_output_interface(
                                &mut self.body,
                                &mut formatting_state,
                                &declarator_list.head.ty,
                                declarator_list.head.name.as_ref(),
                                declarator_list.head.initializer.as_ref(),
                            );
                            for d in declarator_list.tail.iter() {
                                write_output_interface(
                                    &mut self.body,
                                    &mut formatting_state,
                                    &declarator_list.head.ty,
                                    Some(&d.ident.ident),
                                    d.initializer.as_ref(),
                                )
                            }
                        } else if is_input_interface(&declarator_list.head) {
                        } else {
                            // must be a constant or some global variable
                            show_init_declarator_list(&mut self.declarations, declarator_list, &mut formatting_state)
                                .unwrap();
                            writeln!(&mut self.declarations, ";").unwrap();
                        }
                    }
                    ast::DeclarationData::FunctionPrototype(ref proto) => {
                        show_function_prototype(&mut self.declarations, proto, &mut formatting_state).unwrap();
                        writeln!(&mut self.declarations, ";").unwrap();
                    }
                    ast::DeclarationData::Precision(_, _) => {
                        // ignored
                    }
                    ast::DeclarationData::Block(ref block) => {
                        show_block(&mut self.declarations, block, &mut formatting_state).unwrap();
                        writeln!(&mut self.declarations, ";").unwrap();
                    }
                    ast::DeclarationData::Invariant(ref ident) => {
                        // ignored
                    }
                },
                ast::ExternalDeclarationData::FunctionDefinition(ref func) => {
                    show_function_definition(&mut self.function_definitions, func, &mut formatting_state).unwrap();
                }
                ast::ExternalDeclarationData::Preprocessor(ref pp) => {
                    show_preprocessor(&mut self.declarations, pp, &mut formatting_state).unwrap();
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Program codegen
////////////////////////////////////////////////////////////////////////////////////////////////////

// make unique names for:
// - non-interface variables
// - functions
// - UDTs

// Pass 1: rewrite the whole program to make noninterface names & types unique
// second pass: write output interface initializers, rewritten to substitute

// codegen process:
// - insert shader inputs, parameters, etc.
// -

#[cfg(test)]
mod tests {
    use crate::{
        eval::pipeline::{
            codegen::{CodegenContext, NameSubstitutionRewriter},
            program::{Preprocessor, Vfs},
            Binding, Program, SsaName, TypeDesc,
        },
        model::typedesc::{ImageDimension, PrimitiveType, SampledImageType, StructType},
    };
    use glsl_lang::{
        transpiler::glsl::{show_translation_unit, FormattingState},
        visitor::{HostMut, VisitorMut},
    };
    use std::{collections::HashMap, sync::Arc};

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

    /*

    void program_fn_47(vec3 position, vec3 normals, vec3 param, vec3 param2[45], out vec4 normals) {
        // only need to rewrite unbound uniforms
        normals = f(position);
        // issue: what prevents the function to access uniforms directly?
        // nothing => the contents of functions should be rewritten
    }

    void main() {

        // in exprs
        // -> replace all references to bound variables by their bindings
        // -> if not bound, then replace by the uniform name
        //

        // from /root/blur_0/src
        vec4 normals_0 = f__0(position_0);
        vec4 output_0 = normals_0;

    }

     */

    // language=glsl
    const INLINE_EXPR: &str = r#"
    in vec3 color;
    out vec3 o_color = color * vec3(1.0, 2.0, 3.0);
    "#;

    /*
       vec3 node_43_output = node_42_output * vec3(1.0, 2.0, 3.0);
    */

    #[test]
    fn type_mangling() {
        let mut cg = CodegenContext::new();

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

        let mut cg = CodegenContext::new();
        let mut rewriter = NameSubstitutionRewriter {
            cg: &mut cg,
            non_type_name_substitutions: Default::default(),
            type_name_substitutions: Default::default(),
            shadowed_names: Default::default(),
        };
        program.translation_unit.visit_mut(&mut rewriter);

        let mut out = String::new();
        let formatting_state = FormattingState::default();
        show_translation_unit(&mut out, &program.translation_unit, formatting_state).unwrap();
        eprintln!("{}", out);
    }

    /*#[test]
    fn test_codegen() {
        let mut vfs = Vfs::new();
        vfs.register_source("common.glsl", COMMON);
        vfs.dump();
        let mut pp = Preprocessor::new_with_fs(vfs);
        let mut program = Program::new(INLINE_EXPR, "inline_expr", &mut pp).unwrap();
        let mut ctx = CodegenContext::new(460);
        ctx.add_program(
            &program,
            &[Binding {
                var_name: SsaName {
                    base: "node_42_output".into(),
                    index: 0,
                },
                interface_name: "color".into(),
            }],
            &[Binding {
                var_name: SsaName {
                    base: "node_43_output".into(),
                    index: 0,
                },
                interface_name: "o_color".into(),
            }],
        );
        let mut generated = String::new();
        ctx.generate(&mut generated);
        eprintln!("{generated}");
    }*/
}

// GlslCodegen(input VFS)
// - add source
// - add input var
