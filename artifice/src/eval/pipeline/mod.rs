//! GPU pipelines
use kyute::graal::ash::extensions::experimental::amd::GpaSqShaderStageFlags;
use kyute_common::{Atom, Data};
use std::{
    cmp::Ordering,
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

/*/// Pipeline value type.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Data)]
pub enum ValueType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    IVec2,
    IVec3,
    IVec4,
    Mat3,
    Mat4,
}*/

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BuiltinProgramInput {
    // --- Vertex ---
    VertexID,
    InstanceID,
    // --- Fragment ---
    FragCoord,
    FrontFacing,
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

    /// Creates a new environment initialized with the OpenGL vertex stage built-in variables.
    pub fn gl_vertex_builtins() -> VariableCtx {
        let mut vars = imbl::HashMap::new();
        {
            let name: Arc<str> = "gl_VertexID".into();
            vars.insert(
                name.clone(),
                Variable {
                    name: SsaName::new(name, 0),
                    builtin: Some(BuiltinProgramInput::VertexID),
                    variability: Variability::Vertex,
                    ty: TypeDesc::INT,
                },
            );
        }
        {
            let name: Arc<str> = "gl_InstanceID".into();
            vars.insert(
                name.clone(),
                Variable {
                    name: SsaName::new(name, 0),
                    builtin: Some(BuiltinProgramInput::InstanceID),
                    variability: Variability::Vertex,
                    ty: TypeDesc::INT,
                },
            );
        }
        VariableCtx { vars }
    }

    /// Creates a new environment initialized with the OpenGL fragment stage built-in variables.
    pub fn gl_fragment_builtins() -> VariableCtx {
        let mut vars = imbl::HashMap::new();
        {
            let name: Arc<str> = "gl_FragCoord".into();
            vars.insert(
                name.clone(),
                Variable {
                    name: SsaName::new(name, 0),
                    builtin: Some(BuiltinProgramInput::FragCoord),
                    variability: Variability::Fragment,
                    ty: TypeDesc::VEC2,
                },
            );
        }
        {
            let name: Arc<str> = "gl_FrontFacing".into();
            vars.insert(
                name.clone(),
                Variable {
                    name: SsaName::new(name, 0),
                    builtin: Some(BuiltinProgramInput::FrontFacing),
                    variability: Variability::Fragment,
                    ty: TypeDesc::BOOL,
                },
            );
        }
        VariableCtx { vars }
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
    stage: ShaderStage,
}

impl PipelineNode {
    /// Returns a reference to the pipeline variable with the given name.
    pub fn variable(&self, name: impl Into<Arc<str>>) -> Result<&Variable, PipelineError> {
        let name = name.into();
        self.vars.get(&name).ok_or(PipelineError::VariableNotFound)
    }

    pub fn input(stage: ShaderStage, vars: VariableCtx) -> Arc<PipelineNode> {
        Arc::new(PipelineNode {
            parents: Vec::new(),
            vars,
            kind: PipelineNodeKind::Input,
            stage,
        })
    }
}

pub enum PipelineNodeKind {
    Input,
    Program(ProgramNode),
    Interpolation(InterpolationNode),
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
        self.vars.create(out.clone(), var.ty.clone(), Variability::Fragment);
        self.node.vars.push(InterpolatedVariable {
            in_: var.name.base.clone(),
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
            stage: ShaderStage::Fragment,
        })
    }
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

#[derive(Clone, Debug)]
pub struct Binding {
    // TODO replace with an interface index
    pub interface_name: Arc<str>,
    pub var_name: SsaName,
    //variability: Variability,
    //pub ty: TypeDesc,
}

/// A program node in a shader pipeline.
#[derive(Clone)]
pub struct ProgramNode {
    program: Program,
    input_bindings: Vec<Binding>,
    output_bindings: Vec<Binding>,
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
    input_bindings: Vec<Binding>,
    output_bindings: Vec<(Arc<str>, Arc<str>)>,
}

impl ProgramNodeBuilder {
    pub fn new(pred: Arc<PipelineNode>, program: Program) -> Self {
        Self {
            pred,
            program,
            variabilities: Default::default(),
            input_bindings: Default::default(),
            output_bindings: Default::default(),
        }
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
        // check for incompatible variabilities among input bindings
        let vs: Vec<_> = self.variabilities.into_iter().collect();
        let n = vs.len();
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
        }

        // compute minimum variability of the inputs, which defines the variability of the outputs,
        // and also the pipeline stage
        let mut min_variability = Variability::Constant;
        for v in vs {
            if v > min_variability {
                min_variability = v;
            }
        }

        let stage = match min_variability {
            Variability::Vertex => ShaderStage::Vertex,
            Variability::Fragment => ShaderStage::Fragment,
            Variability::Constant
            | Variability::TimeVarying
            | Variability::Material
            | Variability::Object
            | Variability::DrawInstance => self.pred.stage,
        };
        eprintln!("min_variability={:?}, stage={:?}", min_variability, stage);

        // define output variables
        let mut vars = self.pred.vars.clone();
        let mut output_bindings = Vec::new();
        for (interface_name, binding_name) in self.output_bindings {
            let interface = self.program.interface(&interface_name).unwrap();
            let var = vars.create(binding_name, interface.ty.clone(), min_variability);
            output_bindings.push(Binding {
                interface_name,
                var_name: var.name,
            })
        }

        Ok(Arc::new(PipelineNode {
            parents: vec![self.pred],
            vars,
            kind: PipelineNodeKind::Program(ProgramNode {
                program: self.program,
                input_bindings: self.input_bindings,
                output_bindings,
            }),
            stage,
        }))
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
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

        for &node in nodes.iter() {
            match node.kind {
                PipelineNodeKind::Input => {}
                PipelineNodeKind::Program(ProgramNode {
                    ref program,
                    ref input_bindings,
                    ref output_bindings,
                }) => match node.stage {
                    ShaderStage::Vertex => {
                        cgc_vert.add_program(program, input_bindings, output_bindings);
                    }
                    ShaderStage::Fragment => {
                        cgc_frag.add_program(program, input_bindings, output_bindings);
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

        /*for (i, interp) in interpolations.iter().enumerate() {
            cgc_vert.add_output(interp.in_, interp.);
            cgc_frag.add_input(interp.out);
        }*/

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
    use artifice::eval::Variability;

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

        let mut init_vars = VariableCtx::new();
        init_vars.create("position", TypeDesc::VEC3, Variability::Vertex);
        init_vars.create("screenSize", TypeDesc::VEC2, Variability::TimeVarying);
        let init = PipelineNode::input(ShaderStage::Vertex, init_vars);

        let prog_1_node = {
            let mut builder = ProgramNode::build(init, prog_1);
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
        eprintln!("====== Vertex: ====== \n {}", shader.vertex);
        eprintln!("====== Fragment: ====== \n {}", shader.fragment);
    }
}

// TODO:
// - program interfaces can be bound to builtin vars (gl_FragCoord, etc => add builtin vars in VariableCtx)
// - allocate stuff in arenas? most of this stuff is immutable anyway
//      - PROBLEM: TypeDesc can't be stored in arenas, maybe use a simpler typedesc
