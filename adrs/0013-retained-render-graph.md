# 13. Retained render graph

Date: 2021-05-13

## Status

Living Draft

## Context

Currently, in graal, a render graph is created on the fly for every frame. Once submitted, it is discarded.
This is wasteful for multiple reasons:

1. The analysis made in the render graph is not trivial: deducing barriers and aliasing opportunities has a cost.
   We are currently throwing away the analysis results even if the next frame is the same (modulo uniforms).
   This "throwaway" design was chosen based on the assumption that a render graph would be cheap to build.
   However, we don't really know for sure how "cheap" this analysis is for complex pipelines in practice. 

2. With the current design, the analysis only has information about the frame
   being submitted and the previous state of the resources, but nothing about how the resources will be used 
   in the **next** frames; i.e. the analysis doesn't know the future. 
   Not having this information may prevent various forms of "inter-frame" optimizations, or at least makes them more
   complicated.
   
The main challenge is (2), because it prevents us from doing memory re-use across frames easily.


## Existing approaches
Rebuild on every frame (throwaway) OR retain graph across frames (retained).

* Unity render graph: same graph instance, but mutable:
> Render graph execution is a three-step process the render graph system completes, from scratch, every frame.

* Our machinery (High-Level Rendering Using Render Graphs): unclear

* Rendertoy: some crazy lazy computation graph
    * synchronization has some simplifying assumptions though:
        * Textures always go back to the AnyShaderReadSampledImageOrUniformTexelBuffer state in-between ops, possible useless layout transitions
        * no cross-queue stuff

* FrameGraph: throwaway (according to the GDC presentation)
   

## A higher-level render graph 


Kinds of resources:
- transient: discarded on each invocation of the graph
- persistent: the contents of the resource are preserved (and synchronized) between each invocation of the graph
- static (immutable): the contents of the resource never change (meshes, textures)

Issue: combining graphs?
- how many different graphs?
- e.g. optional passes: another graph VS rebuild the main graph?
  -> then you need to worry about syncing resources between graphs, which defeats the purpose
  -> everything is contained in the graph, except things that must survive a graph rebuild (immutable textures)

Q: Issue: things that change
- with throwaway graphs we just pass the things that change when creating the passes.
- with retained graphs we must 

Q: What about serials?
A: frame creation used to borrow the context, but now it would borrow the context for too long
   - the base serial is now passed on invocation, serials in the baked frame are relative

Q: What do we want to handle generically? 
A: Most things
- pipeline creation
- creating and binding uniform buffers

Q: What about presentation?
A: acquire is an operation like another

Q: What about immutable resources?
A: they are generated with graphs as well, but graphs can be "deconstructed" (finalized?) to obtain the final resources
    Those resources don't need to be registered (in fact, they can't, since they won't have any ID associated to them).
    And they probably should be refcounted or something, but that can be done above.
    
Q: Since the memory for frame resources is the same across invocations, we must wait for the previous invocation of 
   a frame to finish before starting the next one.
A: Yes, either:
  - put the serials in the frame object and sync on them before invocation
  - do that explicitly by cycling between Frame -> FrameFuture -> Frame

Q: What about framebuffers, image views, descriptors?
A: Unsure: it could be a layer on top, or directly inside
   => they are "derived resources", and they should have the same lifetime as the pass.

Q: what about dynamic (streamed) buffers (like the vertex data of egui)? who does the upload and when? how is synchronization
   handled? (considering that the size of those buffers may be dynamic)
A: in all likelihood, the buffers can't be registered in advance, since their size is not know when building the frame.
   this means that they must be created and filled during a command callback, and synchronized manually in the pass.

Q: what about resources that "escape" the frame? Like static/immutable resources (textures, mesh data)?
A: everything that can escape the frame is trouble. A solution could be something like this:
```rust
fn main() {
    let frame_builder = Frame::builder();
    // ... build the frame ... 
    let output = frame_builder.create_image(...);
    // ...
    let frame = frame_builder.build();
    
    // Create an invocation object for the frame
    let invocation = frame.invoke();
    // get handles to output resource
    let output_image : GpuFuture<Image> = invocation.get_image(output);
    // this launches the GPU work (`invocation`) and  
    let output_image = output_image.wait();
    
}
```
or, possibly, don't use graphs for initialization of static resources

## Conclusion
Not sure whether it's worth it:
- lots of things to rewrite
- dynamic graphs are harder

Keep the current system. Optimize it, make it more lean, maybe more flexible, and remove anything not related to 
synchro or memory aliasing (framebuffers, descriptors, renderpass stuff all go away; possibly remove swapchains as well)

The only things that we need to address with the current system are:
- memory aliasing with allocations in previous frames
- all issues found during egui integration
- split tracking from resource


## Addressing issues with egui integration:

1. DescriptorImageInfo for samplers
   * out of scope
2. format of the render target can't be easily changed dynamically
   * out of scope
3. lots of boilerplate for pipeline creation
   * out of scope
4. passed target must have the correct image usage flags set
   * out of scope
5. need to specify the correct usage flags for uniform buffers
   * out of scope
6. need to copy/move the pipeline, pipeline layout and render pass before entering the command callback
   * frame builder API simplification
7. create_framebuffer method, generated by DescriptorSetInterface, is not visible by autocompletion
   * out of scope
8. registering accesses to resources manually is a PITA
   * out of scope
8. unreferenced transient resources end up silently deleted before the pass starts
    * this is because transient resources are not really tied to a frame
9. there needs to be something to simplify the creation of render passes and stuff
10. possible confusion between Context and CommandContext
    * API simplification
11. implementing VertexData for a foreign type is sometimes necessary
12. unclear whether it's necessary to register an uniform buffer or not for synchronization
13. BufferData defines len() for references to arrays, which shadows the slice length => this is very sneaky and *must* be eliminated
    * eliminate
14. color blend attachment must match the number of attachments, but they are not specified at the same location
    * out of scope
15. borrowing in Pass command handlers
    * API simplification

## Lifetime of resources

There's a problem with the lifetime of resources returned by `Context::create_image/buffer`:
- if transient (initial refcount = 0), then it will be dropped on the next call to `Frame::finish()` (which calls `Context::cleanup_resources`),
  **unless** the resource has been referenced in a pass.
    - this has been a problem for uniform buffers which technically do not need to be registered (no sync needed, in theory)
- if not transient (initial refcount = 1), then it will not be dropped **until** the user refcount reaches zero and `cleanup_resources` 
    is called.
  
This system was designed to minimize overhead by delegating lifetime management to something on top, but this is still error prone.

First:
- remove the "initial refcount = 0" behavior => always initialize with a non-zero refcount
- thinking about refcounting on top:

```rust
// backref to context -> potentially lots of context refs/unrefs  
struct Image1(Arc<Context>,ImageId);
// backref to refcount block 
struct Image2(Arc<RefCount>,ImageId);
// ref to resource block 
struct Image3(Arc<ImageResource>);
```
Actually, just remove ref counting entirely from the backend

Problem: it is used for aliasing


## Problems with low-level automatic aliasing
- Currently, automatic aliasing only works within a frame
- Transient resource could alias with previous frames, but then you'd need to "keep them alive" longer than necessary:
    - e.g. frame #1 uses and discards resource A, backed by memory block 1
    - frame #2 creates and uses B, which is compatible with memory block 1
        - we need to "extract" the memory resource from the in-flight frame #1 re-use it
        - but that doesn't work if we specify that there must be no frames in flight before submission:
            - in this case, the resources of frame #1 are reclaimed before the resource of #2 can be allocated
            - this results in constant allocations/deallocations
- In any case, a lot of image objects are created for each frame, but they could be reused
    - creating an image is probably not very costly

## Problem with graph-owned resources
- What if a resource needs to be updated at a lower frequency than the screen?
    - e.g. lighting every two frames
    - Option A: rebuild graphs, obviously this ends up being the same as rebuilding the graph every frame
    - Option B: build multiple graphs for each config: the allocated memory ends up being multiplied by the number of different configs
    - Option C: optional passes: more complexity in the backend
    - Option D: build one graph for each feature (lighting, main render, post proc): complexity (sync between graphs)
    
## Proposal: remove low-level automatic aliasing
Move automatic aliasing to a higher level. 
The higher level can have "temporal" knowledge (i.e. it will know how the resource will be used in subsequent frames),
so it can keep resources alive.

Advantages:
- remove resource lifetime tracking
- no need to build the reachability graph
- less complexity during submission
- more reliable, since aliased resources are explicitly tracked

Drawbacks:
- must be implemented by a layer on top
- cannot alias memory blocks anymore
    - must alias resources
    - cannot make two resources with different metadata use the same block of memory
    - note: unsure about how memory aliasing across different resource types is useful in practice
    

     
tradeoff:
- to alias memory across frames, the resource memory must live for longer than the frame
- to alias memory between resources within a frame, it must have global knowledge of the frame
- thus, to alias memory across frames and within a frame, we must have global knowledge of the frame, and the frame must 
  be reused.