// Stages of building a shader:
// - some nodes have inputs connected to shader operators
// - get the shader operator, ask it to produce a shader network?
// - shader value is a *list* of shaders

// Type of shader inputs/outputs?
//

use crate::eval::{pipeline::PipelineNode, EvalError, OpCtx, Variability};
use graal_spirv::typedesc::TypeDesc;
use imbl::HashMap;
use kyute_common::Atom;
use std::{cmp::min, sync::Arc};
use thiserror::Error;
use tracing::warn;

////////////////////////////////////////////////////////////////////////////////////////////////////
//
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Context passed to `OpPipeline` operators.
pub struct OpPipelineCtx {
    op_ctx: OpCtx,
}

/// GPU pipeline operators.
///
/// GPU pipeline operators produce GPU pipelines in the form of a DAG of pipeline nodes.
pub trait OpPipeline {
    fn create_pipeline_node(&self, ctx: &OpPipelineCtx) -> Result<Arc<PipelineNode>, EvalError>;
}

pub struct OpGpuProgram;

//
// Shaders are specified as a DAG of pipeline nodes
// Programs, which are a kind of (pipeline) node

// A stage is one of those: vertex, fragment, geometry, tesselation, compute, etc.
// -> microstage?
// -> program?

/*
/// A program with its inputs and outputs bound to named variables in a pipeline context.
#[derive(Clone, Debug)]
pub struct BoundProgram {
    program: Arc<Program>,
    // name -> name in program
    bindings: HashMap<Atom, PipelineVariable>,
}

impl ShaderNode {
    /// Parses a shader node from GLSL(superset) source code.
    pub fn new(glsl: impl Into<String>) -> Result<ShaderNode, ShaderError> {
        // parse the GLSL into a translation unit
        let mut translation_unit = glsl_lang::ast::TranslationUnit::parse(glsl)
            .map_err(|err| ShaderError::ParseError(format!("{:?}", err)))?;

        // scan inputs & outputs
        for decl in translation_unit.0.iter() {}

        Ok(todo!())
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Data)]
pub enum InterpolationMode {
    Flat,
    NoPerspective,
    Smooth,
}

struct InterpolatedAttribute {
    in_name: Atom,
    out_name: Atom,
    mode: InterpolationMode,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Data)]
pub struct InterpolationNode {
    attributes: InterpolatedAttribute,
}

pub struct ProgramBinding {
    name: Atom,
    variable: PipelineVariable,
}

pub enum PipelineNodeKind {
    /// Shader node (vertex, fragment, etc.)
    Program {
        program: Arc<Program>,
        bindings: HashMap<Atom, ProgramBinding>,
    },
    /// Attribute interpolation
    Interpolation(InterpolationNode),
}

#[derive(Clone, Data)]
pub struct PipelineVariable {
    pub name: Atom,
    pub ssa: usize,
    pub variability: Variability,
}

pub struct ProgramBuilder<'a> {
    pipeline: &'a mut PipelineNode,
    program: Program,
    //bindings: HashMap<Atom, ProgramBinding>,
    inputs: Vec<(Atom, PipelineVariable)>,
    outputs: Vec<(Atom, Atom)>,
    errors: Vec<Atom>,
}

impl<'a> ProgramBuilder<'a> {
    /// Binds a program input by variable name.
    pub fn input(mut self, input_name: Atom, variable: Atom) -> ProgramBuilder<'a> {
        // resolve the input variable
        if let Some(var) = self.pipeline.variables.get(&variable) {
            self.inputs.push((input_name, var.clone()));
        } else {
            warn!("no variable named `{}` in current context", variable);
        }
        self
    }

    pub fn output(mut self, output_name: Atom, variable: Atom) -> ProgramBuilder<'a> {
        self.outputs.push((output_name, variable));
        self
    }

    pub fn finish(mut self) {
        todo!()

        // Check that all inputs have compatible (i.e. comparable) variabilities.
        // This ensures that we're not mixing, e.g., vertex and fragment streams.

        //let input_variabilities : Vec<_> = self.inputs.iter().map(|x| x.1.variability).collect();
        //input_variabilities.so

        //let inferred_output_variability = self.inputs.iter().map(|x| x.1.variability).reduce(|min_variability, v| {
        //
        //})
    }
}

#[derive(Clone)]
pub struct PipelineNode {
    dependencies: Vec<Arc<PipelineNode>>,
    kind: PipelineNodeKind,
    /// Pipeline variables available at this stage.
    variables: HashMap<Atom, PipelineVariable>,
}

impl PipelineNode {
    /// Starts building a program node in this pipeline.
    pub fn build_program(&mut self, program: Program) -> ProgramBuilder {
        ProgramBuilder {
            pipeline: self,
            program,
            bindings: Default::default(),
        }
    }

    /// Appends a shader
    pub fn add_program(&mut self, program: BoundProgram) -> Result<(), ShaderError> {
        program
            .bind(ctx) // ProgramBinder<'a>
            .input(name, name2)
            .output(name, name2)
            .finish();

        // resolve inputs
        //let inputs =
    }
}

fn generate_fragment_shader(pipeline: &PipelineNode, color: &str) {

    // collect all outputs with fragment variability
}

/// Shader
#[derive(Clone, Debug)]
pub struct ShaderPipeline {
    /// Vertex shader nodes.
    vertex: Vec<ShaderNode>,
    /// Fragment shader nodes.
    fragment: Vec<ShaderNode>,
}

#[cfg(test)]
mod tests {
    use glsl_lang::{ast, lexer::v2_min::str::Lexer, parse::Parse};

    #[test]
    fn test_glsl_ast() {
        // Some GLSL source to parse
        // language=glsl
        let source = r#"
        #include <ray_tracing>

        in vec3 position;               // inferred variability
        in vec3 normals;                // inferred variability
        uniform in vec3 param;          // explicit uniform variability
        out vec4 normals = f(position); // inferred variability

        vec4 f() {
            return vec4(0.0,0.0,0.0,1.0);
        }
"#;

        // Try parsing the source
        let ast = ast::TranslationUnit::parse::<Lexer>(source);
        eprintln!("{:?}", ast);
        assert!(ast.is_ok());
    }

    #[test]
    fn test_program_builder() {}
}
*/
