//! GPU pipelines
use kyute::graal::ash::extensions::experimental::amd::GpaSqShaderStageFlags;
use kyute_common::{Atom, Data};
use std::{
    cmp::{min, Ordering},
    collections::{HashMap, HashSet},
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
};
use thiserror::Error;

pub mod codegen;
pub mod layout;
pub mod program;

use crate::eval::{pipeline::codegen::CodegenContext, EvalError, OpCtx, Variability};
pub use crate::model::typedesc::TypeDesc;
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
    #[error("variability mismatch: expected {expected:?}, got {got:?}")]
    VariabilityMismatch { expected: Variability, got: Variability },

    /// Invalid types.
    #[error("type mismatch: expected {expected:?}, got {got:?}")]
    TypeMismatch { expected: TypeDesc, got: TypeDesc },

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

/// Represents a variable in a shader pipeline.
#[derive(Clone)]
pub struct Variable {
    /// name of the variable.
    pub name: SsaName,
    /// Type of the variable.
    pub ty: TypeDesc,
    /// Variability.
    pub variability: Variability,
    /// The built-in program input that this variable represents, if any.
    pub builtin: Option<BuiltinProgramInput>,
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

/// Represents the environment (visible variables) of a pipeline node.
#[derive(Clone)]
pub struct VariableCtx {
    vars: imbl::HashMap<Arc<str>, Variable>,
}

impl VariableCtx {
    pub fn new() -> VariableCtx {
        VariableCtx {
            vars: Default::default(),
        }
    }

    /// Creates a new variable with the given name, possibly shadowing an existing variable.
    pub fn create(&mut self, name: impl Into<Arc<str>>, ty: TypeDesc, variability: Variability) -> Variable {
        let name = name.into();
        self.vars
            .entry(name.clone())
            .and_modify(|var| {
                var.name.index += 1;
                var.ty = ty.clone();
                var.variability = variability;
            })
            .or_insert(Variable {
                name: SsaName::new(name.clone(), 0),
                ty,
                variability,
                builtin: None,
            })
            .clone()
    }

    pub fn get(&self, name: &str) -> Option<&Variable> {
        self.vars.get(name)
    }
}

/// Pipeline node.
pub struct PipelineNode {
    parents: Vec<Arc<PipelineNode>>,
    vars: VariableCtx,
    kind: PipelineNodeKind,
    stage: Option<ShaderStage>,
}

impl PipelineNode {
    /// Returns a reference to the pipeline variable with the given name.
    pub fn variable(&self, name: impl Into<Arc<str>>) -> Result<&Variable, PipelineError> {
        let name = name.into();
        self.vars.get(&name).ok_or(PipelineError::VariableNotFound)
    }

    /*pub fn input(stage: ShaderStage, vars: VariableCtx) -> Arc<PipelineNode> {
        Arc::new(PipelineNode {
            parents: Vec::new(),
            vars,
            kind: PipelineNodeKind::Input,
            stage,
        })
    }*/
}

pub enum PipelineNodeKind {
    Entry,
    Program(ProgramNode),
    Interpolation(InterpolationNode),
}

pub struct PipelineEntryNodeBuilder {
    variabilities: HashSet<Variability>,
    vars: imbl::HashMap<Arc<str>, Variable>,
}

impl PipelineEntryNodeBuilder {
    pub fn new() -> PipelineEntryNodeBuilder {
        PipelineEntryNodeBuilder {
            variabilities: HashSet::new(),
            vars: Default::default(),
        }
    }

    /// Creates a new environment initialized with the OpenGL vertex stage built-in variables.
    pub fn gl_vertex_builtins(&mut self) {
        self.builtin_variable(
            "gl_VertexID",
            TypeDesc::INT,
            Variability::Vertex,
            BuiltinProgramInput::VertexID,
        );
        self.builtin_variable(
            "gl_InstanceID",
            TypeDesc::INT,
            Variability::Vertex,
            BuiltinProgramInput::InstanceID,
        );
    }

    /// Creates a new environment initialized with the OpenGL fragment stage built-in variables.
    pub fn gl_fragment_builtins(&mut self) {
        self.builtin_variable(
            "gl_FragCoord",
            TypeDesc::VEC2,
            Variability::Fragment,
            BuiltinProgramInput::FragCoord,
        );
        self.builtin_variable(
            "gl_FrontFacing",
            TypeDesc::BOOL,
            Variability::Fragment,
            BuiltinProgramInput::FrontFacing,
        );
    }

    fn builtin_variable(
        &mut self,
        name: impl Into<Arc<str>>,
        ty: TypeDesc,
        variability: Variability,
        builtin: BuiltinProgramInput,
    ) {
        let name = name.into();
        self.vars.insert(
            name.clone(),
            Variable {
                name: SsaName::new(name, 0),
                ty,
                variability,
                builtin: Some(builtin),
            },
        );
    }

    pub fn variable(&mut self, name: impl Into<Arc<str>>, ty: TypeDesc, variability: Variability) {
        let name = name.into();
        self.vars.insert(
            name.clone(),
            Variable {
                name: SsaName::new(name, 0),
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
            vars: VariableCtx { vars: self.vars },
            kind: PipelineNodeKind::Entry,
            stage,
        }))
    }
}

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

pub struct InterpolationNode {
    vars: Vec<InterpolatedVariable>,
}

pub struct InterpolationNodeBuilder {
    parent: Arc<PipelineNode>,
    node: InterpolationNode,
    vars: VariableCtx,
}

impl InterpolationNodeBuilder {
    pub fn new(parent: Arc<PipelineNode>) -> InterpolationNodeBuilder {
        // carry over variables that have at least "uniform" variability
        let mut vars = parent.vars.clone();
        vars.vars.retain(|_, v| v.variability <= Variability::DrawInstance);

        InterpolationNodeBuilder {
            parent,
            node: InterpolationNode { vars: vec![] },
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
            return Err(PipelineError::VariabilityMismatch {
                expected: Variability::Vertex,
                got: var.variability,
            });
        }

        let out = out.into();

        let old = self.vars.vars.insert(
            out.clone(),
            Variable {
                name: SsaName::new(out.clone(), 0),
                ty: var.ty.clone(),
                variability: Variability::Fragment,
                builtin: None,
            },
        );
        if old.is_some() {
            // shadowing not allowed here.
            return Err(PipelineError::other("shadowing not allowed"));
        }

        self.node.vars.push(InterpolatedVariable {
            in_: var.name.base.clone(),
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
            kind: PipelineNodeKind::Interpolation(self.node),
            stage: Some(ShaderStage::Fragment),
        })
    }
}

// bunch of Arc<str>
// Binding
// PipelineNode
// InterpolatedVariable
// Variable

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

#[derive(Clone, Debug)]
pub struct Binding {
    // TODO replace with an interface index
    pub interface_name: Arc<str>,
    pub var_name: SsaName,
    //variability: Variability,
    //pub ty: TypeDesc,
}

#[derive(Clone, Debug)]
pub struct Uniform {
    pub interface_name: Arc<str>,
    pub uniform_name: Arc<str>,
}

/// A program node in a shader pipeline.
#[derive(Clone)]
pub struct ProgramNode {
    program: Program,
    input_bindings: Vec<Binding>,
    output_bindings: Vec<Binding>,
    uniforms: Vec<Uniform>,
}

impl ProgramNode {
    pub fn build(pred: Arc<PipelineNode>, program: Program) -> ProgramNodeBuilder {
        ProgramNodeBuilder::new(pred, program)
    }
}

/// Builder for a program node.
pub struct ProgramNodeBuilder {
    pred: Arc<PipelineNode>,
    program: Program,
    variabilities: HashSet<Variability>,
    uniforms: Vec<Uniform>,
    input_bindings: Vec<Binding>,
    output_bindings: Vec<(Arc<str>, Arc<str>)>,
}

impl ProgramNodeBuilder {
    pub fn new(pred: Arc<PipelineNode>, program: Program) -> Self {
        Self {
            pred,
            program,
            uniforms: Default::default(),
            variabilities: Default::default(),
            input_bindings: Default::default(),
            output_bindings: Default::default(),
        }
    }

    /// Exposes a program interface as a pipeline uniform.
    pub fn uniform(
        &mut self,
        interface_name: impl Into<Arc<str>>,
        uniform_name: impl Into<Arc<str>>,
    ) -> Result<(), PipelineError> {
        let interface_name = interface_name.into();

        let interface_var = self
            .program
            .interface(&*interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;

        // check that the variability is at least uniform
        if let Some(variability) = interface_var.variability {
            if variability > Variability::TimeVarying {
                return Err(PipelineError::other("invalid variability for uniform"));
            }
        }

        // check that the interface is not already bound
        for binding in self.input_bindings.iter() {
            if binding.interface_name == interface_name {
                return Err(PipelineError::other("interface already bound"));
            }
        }

        self.uniforms.push(Uniform {
            interface_name,
            uniform_name: uniform_name.into(),
        });

        Ok(())
    }

    /// Binds a pipeline variable to a program input interface.
    pub fn input(
        &mut self,
        interface_name: impl Into<Arc<str>>,
        variable_name: impl Into<Arc<str>>,
    ) -> Result<(), PipelineError> {
        let interface_name = interface_name.into();

        // check that the interface is not already bound
        for binding in self.input_bindings.iter() {
            if binding.interface_name == interface_name {
                return Err(PipelineError::other("interface already bound"));
            }
        }

        let interface_var = self
            .program
            .interface(&*interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;

        let pipeline_var_name = variable_name.into();
        let pipeline_var = self
            .pred
            .vars
            .get(&*pipeline_var_name)
            .ok_or(PipelineError::VariableNotFound)?;

        if interface_var.output {
            return Err(PipelineError::other("expected input"));
        }

        // check that the input type matches the pipeline variable type
        if interface_var.ty != pipeline_var.ty {
            return Err(PipelineError::TypeMismatch {
                expected: interface_var.ty.clone(),
                got: pipeline_var.ty.clone(),
            });
        }

        self.variabilities.insert(pipeline_var.variability);
        self.input_bindings.push(Binding {
            interface_name: interface_name.clone(),
            var_name: pipeline_var.name.clone(),
        });
        Ok(())
    }

    /// Creates a pipeline variable that will be bound to the specified output of the program.
    pub fn output(&mut self, interface_name: &str, variable_name: impl Into<Arc<str>>) -> Result<(), PipelineError> {
        // check that the interface is not already bound
        for (name, _) in self.output_bindings.iter() {
            if &**name == interface_name {
                return Err(PipelineError::other("interface already bound"));
            }
        }

        let interface = self
            .program
            .interface(&*interface_name)
            .ok_or(PipelineError::InterfaceNotFound)?;
        let variable_name = variable_name.into();
        self.output_bindings.push((interface.name.clone(), variable_name));
        Ok(())
    }

    pub fn finish(mut self) -> Result<Arc<PipelineNode>, PipelineError> {
        let vs: Vec<_> = self.variabilities.into_iter().collect();
        let min_variability = check_ordered_variabilities(&vs)?;
        let stage = shader_stage_from_variability(min_variability);

        //eprintln!("min_variability={:?}, stage={:?}", min_variability, stage);

        // define output variables
        let mut vars = self.pred.vars.clone();
        let mut output_bindings = Vec::new();
        for (interface_name, binding_name) in self.output_bindings {
            let interface = self.program.interface(&interface_name).unwrap();

            // create new variable, possibly shadowing another with the same name
            let var_name = vars
                .vars
                .entry(binding_name.clone())
                .and_modify(|var| {
                    // shadowing, increase SSA index
                    var.name.index += 1;
                    var.ty = interface.ty.clone();
                    var.variability = min_variability;
                })
                .or_insert(Variable {
                    // new variable
                    name: SsaName::new(binding_name.clone(), 0),
                    ty: interface.ty.clone(),
                    variability: min_variability,
                    builtin: None,
                })
                .name
                .clone();

            output_bindings.push(Binding {
                interface_name,
                var_name,
            })
        }

        Ok(Arc::new(PipelineNode {
            parents: vec![self.pred],
            vars,
            kind: PipelineNodeKind::Program(ProgramNode {
                program: self.program,
                input_bindings: self.input_bindings,
                output_bindings,
                uniforms: self.uniforms,
            }),
            stage,
        }))
    }
}

// Problem: once a pipeline variable is made available, it is visible in all downstream nodes
// until shadowed by another variable with the same name.
//
// Problem: what should happen when merging two streams with the same variable names?
// e.g. two streams both having the "position" variable
//
// Option A: that's an error
//
// Problem: the way pipelines are constructed right now (iteratively, by adding nodes to an existing "pipeline" object)
// means that nodes may see variables from a sibling branch that was submitted before, even though they are not supposed to.
//
//
// Recursive impl?
// -> no, assume that the graph may be very deep (> 100 nodes)

pub struct CodegenResult {
    pub vertex: String,
    pub fragment: String,
}

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
        let mut cgc_vert = CodegenContext::new(460);
        let mut cgc_frag = CodegenContext::new(460);

        // collect
        let nodes = self.collect();

        for (name, var) in self.vars.vars.iter() {
            for i in 0..=var.name.index {
                cgc_vert.declare(format!("{name}_{i}"));
                cgc_frag.declare(format!("{name}_{i}"));
            }
        }

        /*let mut inputs = Vec::new();
        let mut vertex = Vec::new();
        let mut interpolation = Vec::new();
        let mut fragment = Vec::new();*/

        let mut interpolations = Vec::new();

        let mut vertex_input_location = 0;
        let mut fragment_output_location = 0;

        for &node in nodes.iter() {
            match node.kind {
                PipelineNodeKind::Entry => {
                    match node.stage {
                        Some(ShaderStage::Vertex) => {
                            for var in node.vars.vars.values() {
                                if var.builtin.is_none() {
                                    // SSA index of input should be zero (first instance of the var name)
                                    assert_eq!(var.name.index, 0);
                                    cgc_vert.add_input(var.name.base.clone(), var.ty.clone(), vertex_input_location);
                                    vertex_input_location += 1;
                                }
                            }
                        }
                        Some(ShaderStage::Fragment) => {
                            /*for var in node.vars.vars.values() {
                                if var.builtin.is_none() {
                                    // SSA index of input should be zero (first instance of the var name)
                                    assert_eq!(var.name.index, 0);
                                    cgc_frag.add_input(var.name.base.clone(), var.ty.clone(), fragment_input_location);
                                    fragment_input_location += 1;
                                }
                            }*/
                        }
                        Some(ShaderStage::Compute) => {
                            todo!()
                        }
                        _ => {
                            todo!()
                        }
                    }
                }
                PipelineNodeKind::Program(ProgramNode {
                    ref program,
                    ref input_bindings,
                    ref output_bindings,
                    ref uniforms,
                }) => match node.stage {
                    Some(ShaderStage::Vertex) => {
                        cgc_vert.add_program(program, input_bindings, output_bindings);
                        //cgc_vert.add_input()
                    }
                    Some(ShaderStage::Fragment) => {
                        cgc_frag.add_program(program, input_bindings, output_bindings);
                    }

                    None => {
                        // value will be visible to all stages
                        // TODO ideally the program would be compiled and executed on the CPU,
                        // and the result passed as uniforms.

                        todo!()
                    }
                    _ => {
                        panic!("unexpected stage")
                    }
                },
                PipelineNodeKind::Interpolation(ref interp) => {
                    interpolations.extend_from_slice(&interp.vars);
                }
            }
        }

        for (i, interp) in interpolations.iter().enumerate() {
            cgc_vert.add_output(interp.in_.clone(), interp.ty.clone(), i as u32);
            cgc_frag.add_input(interp.out.clone(), interp.ty.clone(), i as u32);
        }

        let mut vertex = String::new();
        let mut fragment = String::new();

        cgc_vert.generate(&mut vertex);
        cgc_frag.generate(&mut fragment);

        CodegenResult { vertex, fragment }
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

#[cfg(test)]
mod tests {
    use crate::eval::pipeline::{
        program, InterpolationMode, InterpolationNodeBuilder, PipelineNode, Program, ProgramNode, ShaderStage,
        TypeDesc, VariableCtx,
    };
    use artifice::eval::{pipeline::PipelineEntryNodeBuilder, Variability};
    use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
    use std::alloc::System;

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

    #[test]
    fn test_program_nodes() {
        let vfs = program::Vfs::new();
        let mut preprocessor = program::Preprocessor::new_with_fs(vfs);
        let prog_1 = Program::new(PROG_1, "prog1", &mut preprocessor).unwrap();
        let prog_2 = Program::new(PROG_2, "prog2", &mut preprocessor).unwrap();

        let reg = Region::new(&GLOBAL);

        let entry = {
            let mut builder = PipelineEntryNodeBuilder::new();
            builder.gl_vertex_builtins();
            builder.variable("position", TypeDesc::VEC3, Variability::Vertex);
            builder.variable("screenSize", TypeDesc::VEC2, Variability::TimeVarying);
            builder.finish().unwrap()
        };

        let prog_1_node = {
            let mut builder = ProgramNode::build(entry, prog_1);
            builder.input("position", "position").unwrap();
            builder.output("viewPosition", "viewPosition").unwrap();
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
            let mut builder = ProgramNode::build(vs_to_fs, prog_2);
            builder.input("fragCoord", "fragCoord").unwrap();
            builder.input("screenSize", "screenSize").unwrap();
            builder.output("uv", "uv").unwrap();
            builder.finish().unwrap()
        };

        let shader = prog_2_node.codegen_graphics();

        eprintln!("Stats: {:#?}", reg.change());
        eprintln!("====== Vertex: ====== \n {}", shader.vertex);
        eprintln!("====== Fragment: ====== \n {}", shader.fragment);
        //drop(prog_2_node);
    }
}

// TODO:
// - program interfaces can be bound to builtin vars (gl_FragCoord, etc => add builtin vars in VariableCtx)
// - allocate stuff in arenas? most of this stuff is immutable anyway
//      - PROBLEM: TypeDesc can't be stored in arenas, maybe use a simpler typedesc
// - add unique (string) IDs to PipelineNode

// Will have:
// Pipeline operators:
// * get_pipeline_node(&self) -> PipelineNode
//      build a ProgramNode
//      merge all input variable streams
//
//      for each param
//          if bound to variable, then to ProgramNodeBuilder::input
//          if bound to value: register the input uniform
//          if unbound: try automatic bind in variable context (by semantic, or, if no semantic is available, by name)
//
//
// PROBLEM: the uniforms of this node become visible to others

//      if bound -> builder.input(..., ...)
//
//
// * setup_uniforms(&self, uniforms: &mut UniformInterface)
//      -> UniformInterface can set uniform values by name & variability
//      -> internally, it's a map from unique name to offset in some uniform buffer
//      -> built once
//      -> insert_vecXX(interface_name, value)

// (U)camera, (V)position
// (U)camera, (V)position, (V)normals
// (U)camera, (V)position, (V)normals, (V)bitangents
//
