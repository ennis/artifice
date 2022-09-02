//! GPU pipelines
use artifice::{
    eval::pipeline::layout::{std140_align_member, StructLayout},
    model::typedesc::StructType,
};
use kyute::graal::vk;
use kyute_common::{Atom, Data};
use std::{
    cmp::{min, Ordering},
    collections::{HashMap, HashSet},
    fmt,
    fmt::{Display, Formatter, Write},
    sync::Arc,
};
use thiserror::Error;

pub mod codegen;
pub mod layout;
pub mod program;

pub use crate::model::typedesc::TypeDesc;
use crate::{
    eval::{
        pipeline::{
            codegen::{BoundProgram, CodegenContext, GlslVariable},
            layout::Layout,
        },
        EvalError, OpCtx, Variability,
    },
    model::typedesc::{Field, ImageDimension},
};
pub use program::{Program, ProgramError, ProgramInterface};

/// Error produced by ShaderNode.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Could not parse the program source.
    ///
    /// Contains with a generic diagnostic string (may contain multiple errors).
    #[error("parse error(s): \n{0}")]
    ProgramParseError(String),

    ///
    #[error("variable not found")]
    VariableNotFound,

    /// Invalid variability.
    #[error("variability mismatch")]
    VariabilityMismatch,

    /// Invalid types.
    #[error("type mismatch")]
    TypeMismatch,

    ///
    #[error("program interface not found")]
    InterfaceNotFound,

    /// Kitchen sink
    #[error("pipeline error: {0}")]
    Other(String),
}

impl PipelineError {
    pub fn other(msg: impl Into<String>) -> Self {
        PipelineError::Other(msg.into())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

fn check_ordered_variabilities(vs: &[Variability]) -> Result<Variability, PipelineError> {
    let n = vs.len();
    let mut min_variability = Variability::Constant;
    for i in 0..n {
        for j in i + 1..n {
            match vs[i].partial_cmp(&vs[j]) {
                None => {
                    return Err(PipelineError::other(format!(
                        "program inputs have incompatible variability: {:?} and {:?}",
                        vs[i], vs[j]
                    )));
                }
                _ => {}
            }
        }

        if vs[i] > min_variability {
            min_variability = vs[i];
        }
    }
    Ok(min_variability)
}

fn shader_stage_from_variability(v: Variability) -> Option<ShaderStage> {
    match v {
        Variability::Vertex => Some(ShaderStage::Vertex),
        Variability::Fragment => Some(ShaderStage::Fragment),
        Variability::Constant
        | Variability::TimeVarying
        | Variability::Material
        | Variability::Object
        | Variability::DrawInstance => None,
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Variables
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BuiltinProgramInput {
    // --- Vertex ---
    VertexID,
    InstanceID,
    // --- Fragment ---
    FragCoord,
    FrontFacing,
    // --- Compute ---
    NumWorkGroups,
    WorkGroupID,
    LocalInvocationID,
    GlobalInvocationID,
    LocalInvocationIndex,
}

#[derive(Clone, Debug)]
pub struct SsaName {
    pub base: Arc<str>,
    pub index: usize,
}

impl SsaName {
    fn new(base: impl Into<Arc<str>>, index: usize) -> SsaName {
        SsaName {
            base: base.into(),
            index,
        }
    }
}

impl SsaName {
    /// Returns a `Display` type that prints the variable name suffixed with the SSA index.
    pub fn display(&self) -> impl Display + '_ {
        struct SsaIdent<'a>(&'a SsaName);
        impl<'a> Display for SsaIdent<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}_{}", self.0.base, self.0.index)
            }
        }
        SsaIdent(self)
    }
}

/// Represents a variable in a shader pipeline.
#[derive(Clone)]
pub struct Variable {
    /// name of the variable.
    pub(crate) name: Arc<str>,
    pub(crate) ssa_index: u32,
    /// Type of the variable.
    pub(crate) ty: TypeDesc,
    /// Variability.
    pub(crate) variability: Variability,
    /// The built-in program input that this variable represents, if any.
    pub(crate) builtin: Option<BuiltinProgramInput>,
}

impl Variable {
    /*/// Returns a `Display` type that prints the variable name suffixed with the SSA index.
    pub fn cg_ident(&self) -> impl Display {
        struct SsaIdent<'a>(&'a Variable);
        impl<'a> Display for SsaIdent<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}_{}", self.0.name, self.0.ssa)
            }
        }
        SsaIdent(self)
    }*/
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pipeline node
////////////////////////////////////////////////////////////////////////////////////////////////////

type VarMap = imbl::HashMap<Arc<str>, Variable>;

#[derive(Clone, Debug)]
pub enum Binding {
    Default,
    Variable { name: Arc<str>, ssa_index: u32 },
    Uniform { name: Arc<str> },
}

enum PipelineNodeKind {
    Entry,
    Program { program: Program, bindings: Vec<Binding> },
    Interpolation { vars: Vec<InterpolatedVariable> },
}

/// Pipeline node.
pub struct PipelineNode {
    parents: Vec<Arc<PipelineNode>>,
    vars: VarMap,
    kind: PipelineNodeKind,
    stage: Option<ShaderStage>,
}

impl PipelineNode {
    /// Returns a reference to the pipeline variable with the given name.
    pub fn variable(&self, name: impl Into<Arc<str>>) -> Result<&Variable, PipelineError> {
        let name = name.into();
        self.vars.get(&name).ok_or(PipelineError::VariableNotFound)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Entry
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct PipelineEntryNodeBuilder {
    variabilities: HashSet<Variability>,
    vars: VarMap,
}

impl PipelineEntryNodeBuilder {
    pub fn new() -> PipelineEntryNodeBuilder {
        PipelineEntryNodeBuilder {
            variabilities: HashSet::new(),
            vars: Default::default(),
        }
    }

    /// Creates a new environment initialized with the OpenGL vertex stage built-in variables.
    pub fn add_gl_vertex_builtins(&mut self) {
        self.builtin_variable(
            "gl_VertexID".into(),
            TypeDesc::INT,
            Variability::Vertex,
            BuiltinProgramInput::VertexID,
        );
        self.builtin_variable(
            "gl_InstanceID".into(),
            TypeDesc::INT,
            Variability::Vertex,
            BuiltinProgramInput::InstanceID,
        );
    }

    /// Creates a new environment initialized with the OpenGL fragment stage built-in variables.
    pub fn gl_fragment_builtins(&mut self) {
        self.builtin_variable(
            "gl_FragCoord".into(),
            TypeDesc::VEC2,
            Variability::Fragment,
            BuiltinProgramInput::FragCoord,
        );
        self.builtin_variable(
            "gl_FrontFacing".into(),
            TypeDesc::BOOL,
            Variability::Fragment,
            BuiltinProgramInput::FrontFacing,
        );
    }

    fn builtin_variable(
        &mut self,
        name: Arc<str>,
        ty: TypeDesc,
        variability: Variability,
        builtin: BuiltinProgramInput,
    ) {
        self.vars.insert(
            name.clone(),
            Variable {
                name,
                ssa_index: 0,
                ty,
                variability,
                builtin: Some(builtin),
            },
        );
    }

    pub fn add_variable(&mut self, name: impl Into<Arc<str>>, ty: TypeDesc, variability: Variability) {
        let name = name.into();
        self.vars.insert(
            name.clone(),
            Variable {
                name,
                ssa_index: 0,
                ty,
                variability,
                builtin: None,
            },
        );
    }

    pub fn finish(self) -> Result<Arc<PipelineNode>, PipelineError> {
        let vs: Vec<_> = self.variabilities.into_iter().collect();
        let min_variability = check_ordered_variabilities(&vs)?;
        let stage = shader_stage_from_variability(min_variability);

        Ok(Arc::new(PipelineNode {
            parents: vec![],
            vars: self.vars,
            kind: PipelineNodeKind::Entry,
            stage,
        }))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Interpolation
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Data)]
pub enum InterpolationMode {
    Flat,
    NoPerspective,
    Smooth,
}

#[derive(Clone)]
struct InterpolatedVariable {
    in_: Arc<str>,
    out: Arc<str>,
    ty: TypeDesc,
    mode: InterpolationMode,
}

pub struct InterpolationNodeBuilder {
    parent: Arc<PipelineNode>,
    vars: VarMap,
    interpolated: Vec<InterpolatedVariable>,
}

impl InterpolationNodeBuilder {
    pub fn new(parent: Arc<PipelineNode>) -> InterpolationNodeBuilder {
        // carry over variables that have at least "uniform" variability
        let mut vars = parent.vars.clone();
        vars.retain(|_, v| v.variability <= Variability::DrawInstance);

        InterpolationNodeBuilder {
            parent,
            interpolated: vec![],
            vars,
        }
    }

    pub fn interpolate(
        &mut self,
        in_: &str,
        out: impl Into<Arc<str>>,
        mode: InterpolationMode,
    ) -> Result<(), PipelineError> {
        // verify that the input variable exists, that it has the correct variability, and is of the correct type for the given interpolation mode.
        let var = self.parent.variable(in_.clone())?;
        // can only interpolate vertex-varying values (TODO tess shader support)
        if var.variability != Variability::Vertex {
            return Err(PipelineError::VariabilityMismatch);
        }

        let out = out.into();

        let old = self.vars.insert(
            out.clone(),
            Variable {
                name: out.clone(),
                ssa_index: 0,
                ty: var.ty.clone(),
                variability: Variability::Fragment,
                builtin: None,
            },
        );
        if old.is_some() {
            // shadowing not allowed here.
            return Err(PipelineError::other("shadowing not allowed"));
        }

        self.interpolated.push(InterpolatedVariable {
            in_: var.name.clone(),
            ty: var.ty.clone(),
            out,
            mode,
        });
        Ok(())
    }

    pub fn finish(self) -> Arc<PipelineNode> {
        Arc::new(PipelineNode {
            parents: vec![self.parent],
            vars: self.vars,
            kind: PipelineNodeKind::Interpolation {
                vars: self.interpolated,
            },
            stage: Some(ShaderStage::Fragment),
        })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Program node
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Builder for a program node.
pub struct ProgramNodeBuilder {
    pred: Arc<PipelineNode>,
    program: Program,
    variabilities: HashSet<Variability>,
    bindings: Vec<Binding>,
    vars: VarMap,
}

impl ProgramNodeBuilder {
    pub fn new(pred: Arc<PipelineNode>, program: Program) -> Self {
        let num_interface_vars = program.interface().len();
        let vars = pred.vars.clone();
        Self {
            pred,
            program,
            variabilities: Default::default(),
            bindings: vec![Binding::Default; num_interface_vars],
            vars,
        }
    }

    /// Exposes a program interface as a pipeline uniform.
    pub fn bind_uniform(&mut self, interface_name: &str, uniform_name: &str) -> Result<(), PipelineError> {
        let i = self
            .program
            .interface_index(interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;

        let interface_var = &self.program.interface()[i];

        // check that the variability is at least uniform
        if let Some(variability) = interface_var.variability {
            if variability > Variability::TimeVarying {
                return Err(PipelineError::other("invalid variability for uniform"));
            }
        }

        self.bindings[i] = Binding::Uniform {
            name: uniform_name.into(),
        };

        Ok(())
    }

    /// Binds a program input interface to an existing variable.
    pub fn bind(&mut self, interface_name: &str, variable_name: &str) -> Result<(), PipelineError> {
        let i = self
            .program
            .interface_index(interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;

        let ivar = &self.program.interface()[i];
        let pvar = self
            .pred
            .vars
            .get(variable_name)
            .ok_or(PipelineError::VariableNotFound)?;

        // check that the input type matches the pipeline variable type
        if ivar.ty != pvar.ty {
            return Err(PipelineError::TypeMismatch);
        }

        self.variabilities.insert(pvar.variability);
        self.bindings.push(Binding::Variable {
            name: pvar.name.clone(),
            ssa_index: pvar.ssa_index,
        });
        Ok(())
    }

    /// Creates a pipeline variable that will be bound to the specified output of the program.
    pub fn bind_output(&mut self, interface_name: &str, variable_name: &str) -> Result<(), PipelineError> {
        let i = self
            .program
            .interface_index(interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;
        let ivar = &self.program.interface()[i];

        // create new variable, possibly shadowing another with the same name

        // type annotation to avoid E0282 (why does type deduction fail?)
        let pvarname: Arc<str> = Arc::from(variable_name);
        let pvar = self
            .vars
            .entry(pvarname.clone())
            .and_modify(|var| {
                // shadowing, increase SSA index
                var.ssa_index += 1;
                var.ty = ivar.ty.clone();
                //var.variability = min_variability;
            })
            .or_insert(Variable {
                // new variable
                name: pvarname.clone(),
                ssa_index: 0,
                ty: ivar.ty.clone(),
                variability: Variability::Constant, // filled later once all inputs are known
                builtin: None,
            });

        self.bindings[i] = Binding::Variable {
            name: pvarname,
            ssa_index: pvar.ssa_index,
        };
        Ok(())
    }

    pub fn finish(mut self) -> Result<Arc<PipelineNode>, PipelineError> {
        let vs: Vec<_> = self.variabilities.into_iter().collect();
        let min_variability = check_ordered_variabilities(&vs)?;
        let stage = shader_stage_from_variability(min_variability);

        // update the variability of the outputs, now that we know all inputs and deduced their variability
        // TODO: maybe at some point this "builder" API will become useless, replaced by a function that
        // receives inputs & outputs all at once instead of incrementally building them. In this case,
        // this code could be simplified
        for (i, var) in self.program.interface().iter().enumerate() {
            if var.output {
                match &self.bindings[i] {
                    Binding::Variable { name, ssa_index } => {
                        self.vars.get_mut(name).unwrap().variability = min_variability;
                    }
                    _ => {}
                }
            }
        }

        Ok(Arc::new(PipelineNode {
            parents: vec![self.pred],
            vars: self.vars,
            kind: PipelineNodeKind::Program {
                program: self.program,
                bindings: self.bindings,
            },
            stage,
        }))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Codegen
////////////////////////////////////////////////////////////////////////////////////////////////////

const MAX_SETS: usize = 8;

struct BufferBlock {
    block_name: Arc<str>,
    ty: StructType,
    layout: StructLayout,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum ShaderResourceIndex {
    PushConstant {
        offset: u32,
    },
    Descriptor {
        set: u32,
        binding: u32,
        count: u32,
        descriptor_type: vk::DescriptorType,
    },
    NamedUniform {
        offset: u32,
    },
}

/// Fully describes the interface
#[derive(Debug)]
pub struct ShaderResourceInterface {
    by_name: HashMap<Arc<str>, ShaderResourceIndex>,
    current_binding_index: u32,
    current_push_constant_offset: u32,
    current_uniform_buffer_offset: u32,
    num_sets: usize,
    push_constants_size: u32,
}

impl ShaderResourceInterface {
    fn new() -> ShaderResourceInterface {
        ShaderResourceInterface {
            by_name: Default::default(),
            // set #0, binding #0-2 are reserved
            // s0b0: named uniform buffer
            current_binding_index: 3,
            current_push_constant_offset: 0,
            current_uniform_buffer_offset: 0,
            num_sets: 1,
            push_constants_size: 0,
        }
    }

    fn add_uniform(&mut self, name: Arc<str>, ty: TypeDesc) -> ShaderResourceIndex {
        let desc = if !ty.is_opaque() {
            let offset = std140_align_member(&ty, &mut self.current_uniform_buffer_offset).unwrap();
            ShaderResourceIndex::NamedUniform { offset }
        } else {
            match ty {
                TypeDesc::Array { elem_ty, len } => {
                    todo!("array of opaque elements")
                }
                TypeDesc::RuntimeArray(_) => {
                    todo!("runtime arrays")
                }
                TypeDesc::SampledImage(img) => {
                    let binding = self.current_binding_index;
                    self.current_binding_index += 1;
                    ShaderResourceIndex::Descriptor {
                        set: 0,
                        binding,
                        count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                    }
                }
                TypeDesc::Image(img) => {
                    let binding = self.current_binding_index;
                    self.current_binding_index += 1;
                    ShaderResourceIndex::Descriptor {
                        set: 0,
                        binding,
                        count: 1,
                        descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                    }
                }
                TypeDesc::Pointer(_) => {
                    todo!("pointers")
                }
                TypeDesc::Sampler | TypeDesc::ShadowSampler => {
                    let binding = self.current_binding_index;
                    self.current_binding_index += 1;
                    ShaderResourceIndex::Descriptor {
                        set: 0,
                        binding,
                        count: 1,
                        descriptor_type: vk::DescriptorType::SAMPLER,
                    }
                }
                _ => {
                    panic!("unsupported type")
                }
            }
        };
        self.by_name.insert(name, desc);
        desc
    }
}

pub struct CodegenResult {
    pub sri: ShaderResourceInterface,
    pub vertex_shader: String,
    pub fragment_shader: String,
}

// using SSA names
// -> in pipeline context, can shadow existing variables
// -> they end up with an extra index to disambiguate with previous versions of the variable
// -> this is eliminated once in codegen

impl PipelineNode {
    /// Traverses the DAG of pipeline nodes rooted at this node
    fn collect(&self) -> Vec<&PipelineNode> {
        let mut stack = Vec::new();
        let mut visited = HashSet::new();
        let mut sorted = Vec::new();
        stack.push(self);

        while let Some(visit) = stack.pop() {
            sorted.push(visit);
            visited.insert(visit as *const _);
            for parent in visit.parents.iter() {
                if !visited.contains(&(&**parent as *const _)) {
                    stack.push(&**parent);
                }
            }
        }

        sorted.reverse();
        sorted
    }

    pub fn codegen_graphics(&self) -> CodegenResult {
        // collect
        let nodes = self.collect();

        let mut sri = ShaderResourceInterface::new();
        let mut cg_vertex = CodegenContext::new();
        let mut cg_fragment = CodegenContext::new();
        let mut default_uniform_block_members = String::new();
        let mut uniforms = String::new();
        let mut vertex_input_location = 0;
        let mut vertex_output_location = 0;
        let mut vertex_inputs = String::new();
        let mut vertex_outputs = String::new();
        let mut fragment_inputs = String::new();
        let mut fragment_outputs = String::new();

        // Variables: SsaName, TypeDesc
        // IO bindings: SsaName, Arc<str>
        //
        // Name substitutions: Arc<str> -> Arc<str>
        // SRI: Arc<str> -> Index
        //

        // What bothers me:
        // * bindings: if I hear the word one more time I'm gonna throw this all away
        // * generate_glsl: takes HashMap<Arc<str> -> Arc<str>> but doesn't take ownership of anything.
        // * impl Into<Arc<str>> everywhere
        // * so many steps, each requiring a particular bit of information three kilometers away

        // Decisions:
        // * keep the Program type: a parsed program interface is useful
        // * Program should own names & stuff
        // * Assign an index to each program interface so we don't have to carry around interface names
        //      * Bindings now become a linear slice of `InterfaceIndex -> Option<Variable name>` -> hashmap built internally from that
        // * SSA names should only be used in varctx
        //      * Variable has two members: `base_name: &str` and `full_name: &str`
        // * SRI should own its members
        // * use typed arenas:
        //      Arena<PipelineNode>
        //      string arena
        //

        // Results:
        // * SRI
        // * GLSL shader sources

        // PROBLEM: must keep order of declarations:
        // e.g.
        //
        //      #include <defs.h>
        //      uniform MyStruct value;
        //      out MyStruct color;
        //      #include <code.h>
        //
        //      -> code.h *cannot* make references to `color` or `value`

        //
        //    struct Whatever {};
        //    uniform Whatever var;
        //    Whatever get_var() {
        //        return var;
        //    }
        //
        // Will be rewritten to (uniforms before declarations):
        //
        //    layout(...) uniform DefaultUniforms {
        //          Whatever var;       // declaration not written yet!
        //    }
        //    struct Whatever {};
        //    Whatever get_var() {
        //          return var;
        //    }
        //
        // Writing declarations before uniforms present another problem:
        //    struct Whatever {};
        //    Whatever get_var() {
        //          return var;         // var not visible yet!
        //    }
        //    layout(...) uniform DefaultUniforms {
        //          Whatever var;
        //    }
        //
        // The problem lies with the "free uniform block": free uniforms are "moved" into this block,
        // which may be emitted *after* the first use of the uniform.
        //
        // Possible solution:
        // - for functions, forward declare
        // - for constants, ???
        //
        // - don't reorder variables, but how?
        // - reorder dependent declarations before the free uniform block: is it easy? would need to resolve names...
        //
        // FUCK THIS ROTTEN BULLSHIT IT'S FUCKING IMPOSSIBLE TO WORK WITH THIS LANGUAGE
        // For now, output first the types, then the free uniforms, then the functions.

        let write_default_uniform_block_member = |s: &mut String, ty: &TypeDesc, name: &str| {
            let ty_glsl = ty.display_glsl();
            writeln!(s, "    {ty_glsl} {name};").unwrap();
        };
        let write_uniform = |s: &mut String, set: u32, binding: u32, ty: &TypeDesc, name: &str, count: u32| {
            assert_eq!(count, 1, "TODO");
            let ty_glsl = ty.display_glsl();
            writeln!(s, "layout(set={set},binding={binding}) uniform {ty_glsl} {name};").unwrap();
        };
        let write_input = |s: &mut String, location: u32, ty: &TypeDesc, name: &str| {
            let ty_glsl = ty.display_glsl();
            writeln!(s, "layout(location={location}) in {ty_glsl} {name};").unwrap();
        };
        let write_output = |s: &mut String, location: u32, ty: &TypeDesc, name: &str| {
            let ty_glsl = ty.display_glsl();
            writeln!(s, "layout(location={location}) in {ty_glsl} {name};").unwrap();
        };

        for &node in nodes.iter() {
            match node.kind {
                PipelineNodeKind::Entry => {
                    for var in node.vars.values() {
                        // SSA index of input should be zero (first instance of the var name)
                        //assert_eq!(var.name.index, 0);
                        if var.builtin.is_some() {
                            continue;
                        }
                        match var.variability {
                            // vertex input
                            Variability::Vertex => {
                                write_input(&mut vertex_inputs, vertex_input_location, &var.ty, &var.name);
                                vertex_input_location += 1;
                            }
                            // fragment input? should be the output of an interpolation node...
                            Variability::Fragment => {
                                todo!()
                            }
                            Variability::DrawInstance | Variability::Material | Variability::Object => {
                                // not supported yet
                                todo!()
                            }
                            // the rest are uniforms, assume they are visible to all stages
                            _ => {
                                let index = sri.add_uniform(var.name.clone(), var.ty.clone());
                                match index {
                                    ShaderResourceIndex::PushConstant { .. } => {
                                        todo!()
                                    }
                                    ShaderResourceIndex::Descriptor {
                                        set,
                                        binding,
                                        count,
                                        descriptor_type,
                                    } => {
                                        write_uniform(&mut uniforms, set, binding, &var.ty, &var.name, 0);
                                    }
                                    ShaderResourceIndex::NamedUniform { .. } => {
                                        write_default_uniform_block_member(
                                            &mut default_uniform_block_members,
                                            &var.ty,
                                            &var.name,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                PipelineNodeKind::Program {
                    ref program,
                    ref bindings,
                } => {
                    for (i, b) in bindings.iter().enumerate() {
                        match b {
                            Binding::Default => {}
                            Binding::Variable { name, ssa_index } => {}
                            Binding::Uniform { name } => {
                                let ivar = &program.interface()[i];
                                let sridx = sri.add_uniform(name.clone(), ivar.ty.clone());
                                match sridx {
                                    ShaderResourceIndex::PushConstant { .. } => {
                                        todo!()
                                    }
                                    ShaderResourceIndex::Descriptor {
                                        set, binding, count, ..
                                    } => {
                                        write_uniform(&mut uniforms, set, binding, &ivar.ty, &name, count);
                                    }
                                    ShaderResourceIndex::NamedUniform { .. } => {
                                        write_default_uniform_block_member(
                                            &mut default_uniform_block_members,
                                            &ivar.ty,
                                            name,
                                        );
                                    }
                                }
                                //substitutions.insert(ivar.clone(), uniform.var_name.clone());
                            }
                        }
                    }

                    match node.stage {
                        Some(ShaderStage::Vertex) => {
                            cg_vertex.write_program(program, bindings);
                        }
                        Some(ShaderStage::Fragment) => {
                            cg_fragment.write_program(program, bindings);
                        }
                        None => {
                            todo!()
                        }
                        _ => {
                            panic!("unexpected stage")
                        }
                    }
                }
                PipelineNodeKind::Interpolation { ref vars } => {
                    for v in vars.iter() {
                        let ty_glsl = v.ty.display_glsl();
                        let vertex_output_name = &v.in_;
                        let frag_input_name = &v.out;
                        let mode = match v.mode {
                            InterpolationMode::Flat => "flat",
                            InterpolationMode::NoPerspective => "noperspective",
                            InterpolationMode::Smooth => "smooth",
                        };
                        writeln!(
                            vertex_outputs,
                            " layout(location={vertex_output_location}) {mode} out {ty_glsl} {vertex_output_name};"
                        )
                        .unwrap();
                        writeln!(
                            fragment_inputs,
                            " layout(location={vertex_output_location}) {mode} in {ty_glsl} {frag_input_name};"
                        )
                        .unwrap();
                        vertex_output_location += 1;
                    }
                }
            }
        }

        // final generation step
        let mut free_uniform_block = String::new();
        writeln!(
            free_uniform_block,
            "layout(set=0,binding=0,std140) FreeUniforms {{\n {default_uniform_block_members}}}"
        )
        .unwrap();

        // write the final shaders in this order:
        //
        //    Type declarations
        //    Free uniforms block
        //    Uniforms
        //    Inputs
        //    Outputs
        //    Non-type declarations (functions & constants)
        //
        let write_shader = |cg: &CodegenContext, inputs: &str, outputs: &str| -> String {
            let mut source = String::new();
            let declarations = &cg.declarations;
            let functions = &cg.function_definitions;
            let body = &cg.body;
            writeln!(
                source,
                "{declarations}\n\
                 {free_uniform_block}\n\
                 {uniforms}\n\
                 {inputs}\n\
                 {outputs}\n\
                 {functions}\n\
                 \
                 \
                 void main() {{\n{body}}}
                 "
            )
            .unwrap();
            source
        };

        let vertex_shader = dbg!(write_shader(&cg_vertex, &vertex_inputs, &vertex_outputs));
        let fragment_shader = dbg!(write_shader(&cg_fragment, &fragment_inputs, &fragment_outputs));

        CodegenResult {
            vertex_shader,
            fragment_shader,
            sri,
        }
    }
}

// Preprocessing before generating the shaders:
// - determine the interfaces between stages: locations, and interpolation modes
//
// To generate the (fragment) shader main function:
// - there are two code bodies: the "declaration body" containing the function declarations, and the "main body" consisting of the statements in the main function.
// - program node:
//   - for each input in the program interface, initialize with the provided bindings:
//
//       $type $name_$fileid = $input_binding;
//
//   - for each output: just paste the initializer, and copy into the output variable
//
//       $output_binding = $initializer_expr;
//
//
//   - for each declaration: check if the decl was already processed, otherwise output it in the declaration body, and mark the decl as processed.
//
// - interpolation node: add an item to the interpolation block, e.g.
//
//       layout(location = $interp_counter) smooth in vec3 fragColor;
//
// - input node (generated by scene filters, etc.)
//       add an input attribute

/*/// Program pipeline operators.
#[async_trait]
pub trait OpProgram {
    fn create_pipeline_node(&self, ctx: &OpCtx) -> Result<Arc<PipelineNode>, EvalError>;
}*/

// shader snippets:
// - add dithering

#[cfg(test)]
mod tests {
    use crate::eval::pipeline::{
        program, InterpolationMode, InterpolationNodeBuilder, PipelineNode, Program, ProgramNodeBuilder, ShaderStage,
        TypeDesc,
    };
    use artifice::eval::{pipeline::PipelineEntryNodeBuilder, Variability};
    use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
    use std::{alloc::System, sync::Arc};

    #[global_allocator]
    static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

    const PROG_1: &str = r#"
        in vec3 position;
        uniform mat4 viewMatrix;
        out vec2 viewPosition = (viewMatrix * vec4(position,1.0)).xyz;
        "#;

    const PROG_2: &str = r#"
        in vec2 fragCoord;
        uniform vec2 screenSize;
        out vec2 uv = fragCoord / screenSize;
        "#;

    const COLOR_FROM_FRAG_COORD: &str = r#"
        in vec2 fragCoord;
        out vec4 color = vec4(fragCoord.xy, 0.0, 1.0);
        "#;

    const DITHERING: &str = r#"
    uniform texture2D blueNoiseTex;
    in vec4 color;
    in vec2 fragCoord;
    uniform vec2 screenSize;
    
    vec4 bluenoise(vec2 fc) {
        return texture(blueNoiseTex, fc / textureSize(blueNoiseTex));
    }
    
    out vec4 o_color = color + bluenoise(fragCoord);
    "#;

    /*

    in vec3 position;

    layout(set=0,binding=0) uniform NamedUniforms {
        mat4 viewMatrix_0;
        vec2 screenSize_1;
    };

    layout(set=0,binding=1) uniform sampler2D blueNoiseTex_0;

    */

    // idea: do not bind outputs to separate variables: just create a var with the same name as the interface

    #[test]
    fn test_program_nodes() {
        let vfs = program::Vfs::new();
        let mut preprocessor = program::Preprocessor::new_with_fs(vfs);
        let prog_1 = Program::new(PROG_1, "prog1", &mut preprocessor).unwrap();
        let prog_2 = Program::new(PROG_2, "prog2", &mut preprocessor).unwrap();
        let dithering = Program::new(DITHERING, "dithering", &mut preprocessor).unwrap();
        let color_from_frag_coord =
            Program::new(COLOR_FROM_FRAG_COORD, "color_from_frag_coord", &mut preprocessor).unwrap();

        let reg = Region::new(&GLOBAL);

        let entry = {
            let mut builder = PipelineEntryNodeBuilder::new();
            builder.add_gl_vertex_builtins();
            builder.add_variable("position", TypeDesc::VEC3, Variability::Vertex);
            builder.add_variable("screenSize", TypeDesc::VEC2, Variability::TimeVarying);
            builder.finish().unwrap()
        };

        let prog_1_node = {
            let mut builder = ProgramNodeBuilder::new(entry, prog_1);
            builder.bind("position", "position").unwrap();
            builder.bind_uniform("viewMatrix", "viewMatrix").unwrap();
            builder.bind_output("viewPosition", "viewPosition").unwrap();
            builder.finish().unwrap()
        };

        let vs_to_fs = {
            let mut builder = InterpolationNodeBuilder::new(prog_1_node);
            builder
                .interpolate("viewPosition", "fragCoord", InterpolationMode::Smooth)
                .unwrap();
            builder.finish()
        };

        let prog_2_node = {
            let mut builder = ProgramNodeBuilder::new(vs_to_fs, prog_2);
            builder.bind("fragCoord", "fragCoord").unwrap();
            builder.bind("screenSize", "screenSize").unwrap();
            builder.bind_output("uv", "uv").unwrap();
            builder.finish().unwrap()
        };

        let color_from_frag_coord_node = {
            let mut builder = ProgramNodeBuilder::new(prog_2_node, color_from_frag_coord);
            builder.bind("fragCoord", "fragCoord").unwrap();
            builder.bind_output("color", "color").unwrap();
            builder.finish().unwrap()
        };

        let dithering_node = {
            let mut builder = ProgramNodeBuilder::new(color_from_frag_coord_node, dithering);
            builder.bind_uniform("blueNoiseTex", "blueNoiseTex").unwrap();
            builder.bind("screenSize", "screenSize").unwrap();
            builder.bind("color", "color").unwrap();
            builder.bind_output("o_color", "color").unwrap();
            builder.finish().unwrap()
        };

        let shader = dithering_node.codegen_graphics();

        eprintln!("Stats: {:#?}", reg.change());
        eprintln!("====== Vertex: ====== \n {}", shader.vertex_shader);
        eprintln!("====== Fragment: ====== \n {}", shader.fragment_shader);
        eprintln!("====== SRI: ====== \n {:#?}", shader.sri);
        //drop(prog_2_node);
    }
}
