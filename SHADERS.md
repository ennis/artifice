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
