# Writing shader code in artifice

## Use cases

- Different shader variants depending on whether a vertex attribute is present.
- Drag-drop a node to compute normals in vertex space
- Able to access all buffers in the input bus
  - vertex streams 
- Helper types shared between shaders
  - Globally available shader modules

```
import ray_tracing;

in float3 position;               // inferred variability
vertex in float3 normals;         // explicit variability
uniform in float3 param;          // explicit variability (uniform)
out float4 normals = f(position); // inferred variability

// all parameters must be passed to the function explicitly so that variability can be inferred

// functions
// -> GLSL?
float4 f(...) {
}

// requires that a compatible stream is present in the input bus:
// - shader varying named position
// the "normals" output is inferred to be of the same variability as position

// All of this is can be packaged in an Op, with one I/O bus, that augments the bus with a new stream named `normals`.
// It's not really a shader by itself, it's just a bit of code that derives a value from others.
// Could be executed on the CPU as well.
```

Format of shader snippets:
- how do we specify input arguments?

Option A: only a function? a function by itself doesn't produce a value, it has to be called.

After all shader ops are collected in the context, we get:
- a vertex shader (all nodes that come before a rasterizer node)
- a fragment shader

A shader contains:
- a list of uniforms, bound to values in the input bus
- a list of input variables, which are expected to be provided by 


A complete rasterization pipeline would look like that:

Scene loader
 |
 |   Scene data (variability: time)
 V
Scene filter
 |
 |   Filtered scene data, draw calls, instances (variability: draw instance)
 V
Vertex input (expand to vertex streams)
 |
 |   Bunch of vertex streams (variability: vertex), per-instance values, per-object values, uniforms (time variant)
 V
(zero or more vertex shaders)
 |
 |
 V
Rasterizer (primitive assembly, rasterization and interpolation)
 |
 |   Fragment streams
 V
(zero or more fragment shaders)
 |
 |   Fragment streams
 V
Fragment output
 |
 |   Output images (variability: time)
 V
... (compositing stages?) ...

## Values in the input bus

namespace:name (provenance)

## Goals
Goal: reduce "useless code" to a minimum. What is useless code?
- shader binding declarations (e.g. `layout(...) uniform vec4 whatever`)
- varying declarations

Code should be written _in context_ of something. 
Code doesn't determine the shader interface.

However, shader code can conform to a specific interface, which is defined elsewhere.

## Options
Ideally: a shader language that supports modules loaded from memory, in/out variables, arbitrary attributes, compilable to GPU or CPU.

- GLSL snippets, with custom syntax for I/O
- something else?

## Examples

```
// vertex2d

in vec2 position;
uniform mat3 transform;
out vec2 texcoord;

out vec4 transformedPosition = vec4((transform*vec3(pos,1.0)).xy, 0, 1);

// positions are going to be in clipspace already, do [-1,-1]->[1,1]
in vec2 pos;
out vec2 texcoord;

uniform mat3 transform;

void main() {
gl_Position = vec4((transform*vec3(pos,1.0)).xy, 0, 1);
}
```

```
// clipSpace2d
in vec2 position;
out vec4 position = vec4(position, 0 , 1);
```

By default, inputs bind to a value of the same name in the input bus.




So far, the idea seems to have every PipelineOp produce a PipelineNode, which is a standalone object representing a graph of composed shaders.

Q: Is it worth composing individual "shader snippets" like that, instead of providing a complete shaders? What are the use cases?
A:
* composing shadertoy snippets intuitively? No -> should be composing closures
  * anything that requires information from upwards (like the upstream transform) won't work
* composing simple filters
  * not sure that's better than just writing GLSL directly
* calculating common values, like the post-transform positions or normals
  * again, what about providing a GLSL function for that?
-> it's useful to build the first version of a shader, but does not really reduce iteration times after that


Q: what is there to compose?
A: 
* layers in a compositing / painting application 
* image filters
* procedural generation functions (shadertoy-like stuff)
 
* Composition of image filters fail when it requires neighborhoods, which is like 99% of filters
  * Need closures instead, and in most cases it's cheaper to do several passes
  * Note that programs can be treated as closures as well
    * But with a different composition procedure



Q: does it solve the shader variants problem?
A: No

Conclusion:
Currently, the PipelineNode system is mostly useful as an implementation detail.
In order for it to be more useful, it should support different composition scenarios (for example, shadertoy-like procedural generation functions, "SDF toolkit").

Currently, only one form of composition is supported: sequencing of shader nodes.

Evolution:
* Treat programs as functions, be more flexible in the way we can compose programs, by directly manipulating the AST.
* Convert GLSL AST into our "lightweight" AST (or bytecode), and compose bits of AST to form a program.
* Support closures
* Everything arena-based

Example:

```
let sd_sphere = Function::new(&arena, "...");
let sd_rounded_cylinder = Function::new(&arena, "..."); 
// build another function from both

let op_scale = Function::new(r#"
float opScale(in vec3 p, in float s, in sdf3d primitive) {
    return primitive(p/s)*s;
}
"#); 

let p = Term::vec3(&arena);
let t = Term::new(&arena, transform_ty);
```

Conclusion 2:
* Convert to our own AST, store everything in a dropless arena
* Support closures

Idea:
A "light-weight" version of OpenShadingLanguage. 

Q: what's the name for this evolution?
artifice shader language, arsl

Goals:
* easy composition outside of the language itself
  * the language doesn't matter much, the API around it does
* syntax: GLSL inspired
  * could probably translate straight GLSL to our language

* Would a rowan-based parser be useful?
  * In the long term, why not?

```
type FragColor = closure (
        fragment vec2 fragCoord,
        output fragment vec4 color);

vec4 bluenoise(vec2 fc) {
    return texture(blueNoiseTex, fc / textureSize(blueNoiseTex));
}

void main() {
    o_color = bluenoise(fragCoord);
}
    
void mix_dither(
    fragment vec2 fragCoord,
    texture2D blueNoiseTex,
    FragColor inColor,
    fragment out vec4 outColor 
)
{
    // sample blue noise
    let noise = sample(blueNoiseTex, fragCoord / textureSize(blueNoiseTex), NEAREST);
    outColor = inColor(fragCoord) * noise; 
    
    mix_dither(fragCoord, blueNoiseTex, out outColor);
}
```


Q: Rowan or LALRPOP?
A: The most important thing is the AST (in-memory representation), which is independent of the language. Can think of it as bytecode.
Must be compact, easy to compose, with utilities to extract shader interfaces from it.

Does something like that already exists?


## Alternative: supercharge "program" representations
* rename Program -> Function
* compose and create programs
 
```
fn test()
{
    let t_frag_coord = Term::new("fragCoord");
    
    // program: fn(params, vec2 fragCoord) -> vec4
    // expr: vec4
    let mut expr = program
        .bind(Term::constant(4.0))
        .bind(
    
    let mut program = Program::new(vec![t_frag_coord]);
    
    
}
```

## Is it even worth to emit GLSL anymore?
We can probably emit SPIR-V directly.

The workflow becomes:
- parse GLSL (or anything else, really) into ashley AST
- combine ashley AST from different sources
- dump SPIR-V + shader resource interface from the AST

Possible unknowns:
* emitting SPIR-V headers
  * types: OK (we already know how to parse them, so dumping them from a TypeDesc should be straightforward)
  * control flow: dunno
  * phi nodes? don't generate them, maybe use spirv-opt (see Local Store/Load Elimination - Multiple Store) 
* closure types
  * not a priority?
  * not supported directly in GLSL

Advantages:
* can combine programs from multiple sources, in any language
* 