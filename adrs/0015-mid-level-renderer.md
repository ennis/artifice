# 14. Mid-level renderer (MLR)

Date: 2021-11-17

## Status

Draft

## Context

Graal handles synchronization and memory aliasing on top of vulkan, but doesn't alleviate all pains of doing stuff 
with a low-level graphics API like vulkan. We still need to handle:
- shader: compilation to SPIR-V, creation of modules, specialization
- creation of VkGraphicsPipelines
    - ... and renderpasses
    - with color attachments that match what's in the shader
    - vertex layouts
    - ...
- descriptors: an absolute mess
    - creation and management of descriptor set layouts
    - allocation of descriptor sets
    - bindless boilerplate
- sampler objects: creation and reuse
- uniforms: transient upload buffers, binding, binding offsets
- image views: creation and reuse

We propose another layer on top of graal, the *mid-level renderer* (MLR) that abstracts away all those things, so that 
the user can focus on writing GPU algorithms with minimal boilerplate and ceremony. It would be API-agnostic, even if
initially implemented only atop graal.


## Overview

What MLR **is not**:
- a scene graph
- a reactive / incremental computation framework: 
- a nodal graphics pipeline representation
- an animation system: there's no built-in concept of "time" or "frame"


## Basic concepts
- data: typed buffers and images
  - their location (host or device memory) is abstracted 
- code: shaders / expressions
  - some expressions run only in certain contexts (e.g. derivatives in fragment shader, etc.)
    - execution models

- ultimately, can be used as a scripting language?
  - basically a scripting language that supports 

```rust
struct Vertex3 {
    pos: f32x4,
    color: u8x4
}

fn main() {
  let color_output = image2D(1024, 768);
  let vertices = [Vertex3(0.0, 1.0, 0.0)];

  fn screen_pass(size: Size, frag_inputs: T, #[fragment] fragment_shader: fn(Vertex, T) -> f32x4) -> Image
  {
    fn vertex_shader() -> Vertex3 {
      
    }
    
    // internally defined
    // -> to `render` fragment_shader is not a function pointer, it's an object containing the AST of the function, along
    //    with cached lowerings to GLSL source code
    render(
        
    );
  }
}

```