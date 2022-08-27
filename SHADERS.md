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


# Mixing GPU and CPU ops

Right now CPU ops are run in tasks scheduled by tokio: tasks are spawned to evaluate values & recursively by ops
to evaluate their inputs.
However, this doesn't work with GPU work: GPU work needs to be collected in a "frame" **in advance** (in order for resource aliasing & autosync to work)
i.e. we can't schedule GPU work within a tokio task on-the-fly.

Key points:
* tasks can't schedule new GPU work after the task has started

## Is it possible to dynamically schedule GPU work?
Example: a CPU op needs to schedule some GPU work. It retrieves the current frame object, and adds some work. Then it awaits the result (waits for availability on the CPU).
Problem: the GPU work needs to be kicked off to the GPU device at some point.
Idea: give CPU tasks more priority and evaluate everything on the CPU at first; then, once all CPU work is blocked or running, kick off the GPU work ("flush the frame").

Problem: graal frames borrow the context mutably: may be difficult to weave that into async tasks.
Problem: async tasks spawned on the tokio runtime must be 'static, so absolutely no borrowing possible.

Workaround: within a single task, it's possible to borrow the GPU context:
So: 
* spawn a "GPU frame" task, with a mpsc channel
* GPU work is sent over that channel (work items are 'static)
* when the GPU frame task wakes up:
  * fetch work items from the channel and add them to the list
  * if no work:
    * create frame, add work items, push it to the GPU
* Actually, no need for the channel, just push GPU work items to the shared state

Alternatively, we could make `graal::Frame` a standalone type that doesn't borrow the context, and submit the whole frame at once.

# What's the API for GPU image evaluation?

Q: What does eval return?
A: Most likely, a handle to a GPU image (the access to which should be synchronized)
Q: a `graal::ImageId`?
A: maybe a RAII wrapper over that, because otherwise the ownership is not clear.
Q: should this wrapper also handle CPU-stored images?
A: let's try that

Q: value or reference semantics? 
A: have to remember how graal does resource aliasing: the resources have to be destroyed as soon as possible, *in the frame being built*



