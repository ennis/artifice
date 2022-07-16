//! GPU pipelines
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

use crate::eval::Variability;
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
    #[error("variable not found: {0}")]
    VariableNotFound(Atom),

    /// Invalid variability.
    #[error("variability mismatch: expected {expected:?}, got {got:?}")]
    VariabilityMismatch { expected: Variability, got: Variability },

    /// Invalid types.
    #[error("type mismatch: expected {expected:?}, got {got:?}")]
    TypeMismatch { expected: TypeDesc, got: TypeDesc },

    ///
    #[error("program interface not found: {0}")]
    InterfaceNotFound(Atom),

    /// Kitchen sink
    #[error("pipeline error: {0}")]
    Other(String),
}

impl PipelineError {
    pub fn other(msg: impl Into<String>) -> Self {
        PipelineError::Other(msg.into())
    }
}

/// Pipeline value type.
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
}

/// Represents a variable in a shader pipeline.
#[derive(Clone)]
pub struct Variable {
    /// Base name of the variable.
    pub name: Atom,
    /// SSA index of the variable.
    pub ssa: usize,
    /// Type of the variable.
    pub ty: TypeDesc,
    /// Variability.
    pub variability: Variability,
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

#[derive(Clone)]
pub struct VariableCtx {
    vars: imbl::HashMap<Atom, Variable>,
}

impl VariableCtx {
    pub fn new() -> VariableCtx {
        VariableCtx {
            vars: Default::default(),
        }
    }

    /// Creates a new variable with the given name, possibly shadowing an existing variable.
    pub fn create(&mut self, name: impl Into<Atom>, ty: TypeDesc, variability: Variability) -> Variable {
        let name = name.into();
        self.vars
            .entry(name.clone())
            .and_modify(|var| {
                var.ssa += 1;
                var.ty = ty.clone();
                var.variability = variability;
            })
            .or_insert(Variable {
                name: name.clone(),
                ssa: 0,
                ty,
                variability,
            })
            .clone()
    }
}

/// Pipeline node.
pub struct PipelineNode {
    parents: Vec<Arc<PipelineNode>>,
    vars: VariableCtx,
    kind: PipelineNodeKind,
}

impl PipelineNode {
    /// Returns a reference to the pipeline variable with the given name.
    pub fn variable(&self, name: impl Into<Atom>) -> Result<&Variable, PipelineError> {
        let name = name.into();
        self.vars
            .vars
            .get(&name)
            .ok_or(PipelineError::VariableNotFound(name.clone()))
    }

    pub fn input(vars: VariableCtx) -> Arc<PipelineNode> {
        Arc::new(PipelineNode {
            parents: Vec::new(),
            vars,
            kind: PipelineNodeKind::Input,
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

struct InterpolatedVariable {
    in_: Atom,
    out: Atom,
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
    pub fn interpolate(&mut self, in_: Atom, out: Atom, mode: InterpolationMode) -> Result<(), PipelineError> {
        // verify that the input variable exists, that it has the correct variability, and is of the correct type for the given interpolation mode.

        let var = self.parent.variable(in_.clone())?;
        // can only interpolate vertex-varying values (TODO tess shader support)
        if var.variability != Variability::Vertex {
            return Err(PipelineError::VariabilityMismatch {
                expected: Variability::Vertex,
                got: var.variability,
            });
        }

        self.vars.create(out.clone(), var.ty.clone(), Variability::Fragment);
        self.node.vars.push(InterpolatedVariable { in_, out, mode });
        Ok(())
    }

    pub fn finish(self) -> Arc<PipelineNode> {
        Arc::new(PipelineNode {
            parents: vec![self.parent],
            vars: self.vars,
            kind: PipelineNodeKind::Interpolation(self.node),
        })
    }
}

/// A program node in a shader pipeline.
#[derive(Clone)]
pub struct ProgramNode {
    program: Program,
    input_bindings: HashMap<Atom, Variable>,
    output_bindings: HashMap<Atom, ProgramInterface>,
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
    input_bindings: HashMap<Atom, Variable>,
    output_bindings: HashMap<Atom, ProgramInterface>,
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
        interface_name: impl Into<Atom>,
        variable_name: impl Into<Atom>,
    ) -> Result<(), PipelineError> {
        let interface_name = interface_name.into();
        let interface_var = self
            .program
            .interface(interface_name.clone())
            .ok_or(PipelineError::InterfaceNotFound(interface_name.clone()))?;
        let pipeline_var_name = variable_name.into();
        let pipeline_var = self.pred.variable(pipeline_var_name.clone())?;

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
        self.input_bindings
            .insert(interface_var.name.clone(), pipeline_var.clone());
        Ok(())
    }

    /// Creates a pipeline variable that will be bound to the specified output of the program.
    pub fn output(
        &mut self,
        interface_name: impl Into<Atom>,
        variable_name: impl Into<Atom>,
    ) -> Result<(), PipelineError> {
        let interface_name = interface_name.into();
        let interface_var = self
            .program
            .interface(interface_name.clone())
            .ok_or(PipelineError::InterfaceNotFound(interface_name.clone()))?;

        if !interface_var.output {
            return Err(PipelineError::other("expected output"));
        }

        self.output_bindings.insert(interface_name, interface_var.clone());

        Ok(())
    }

    pub fn finish(mut self) -> Result<Arc<PipelineNode>, PipelineError> {
        // infer output variabilities
        // create output variables

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

        // compute minimum variability of the inputs, which defines the variability of the outputs
        let mut min_variability = Variability::Constant;
        for v in vs {
            if v < min_variability {
                min_variability = v;
            }
        }

        // create output varctx
        let mut vars = self.pred.vars.clone();
        for (binding, output) in self.output_bindings.iter() {
            vars.create(binding.clone(), output.ty.clone(), min_variability);
        }

        Ok(Arc::new(PipelineNode {
            parents: vec![self.pred],
            vars,
            kind: PipelineNodeKind::Program(ProgramNode {
                program: self.program,
                input_bindings: self.input_bindings,
                output_bindings: self.output_bindings,
            }),
        }))
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

#[cfg(test)]
mod tests {
    use crate::eval::pipeline::{program, PipelineNode, Program, ProgramNode, TypeDesc, VariableCtx};
    use artifice::eval::Variability;

    const PROG_1: &str = r#"
        in vec3 position;
        uniform mat4 viewMatrix;
        out vec3 viewPosition = (viewMatrix * vec4(position,1.0)).xyz;
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
        let init = PipelineNode::input(init_vars);

        let prog_1_node = {
            let mut builder = ProgramNode::build(init, prog_1);
            builder.input("position", "position").unwrap();
            builder.output("viewPosition", "viewPosition").unwrap();
            builder.finish().unwrap()
        };

        let prog_2_node = {
            let mut builder = ProgramNode::build(prog_1_node, prog_2);
            builder.input("fragCoord", "fragCoord").unwrap();
            builder.input("screenSize", "screenSize").unwrap();
            builder.output("uv", "uv").unwrap();
            builder.finish().unwrap()
        };

        let term = {};
    }
}
