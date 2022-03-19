Artifice
====================================



This is the documentation for Graal, a convenience layer on top of the Vulkan API that exposes a minimal "frame graph" abstraction.

Graal is designed to facilitate working with the Vulkan API by automating or reducing the boilerplate of several tasks:
.. FIXME: it's not only that: there's also functionality to automatically generate descriptor set layouts, render passes, vertex attribute descriptions...

* **Synchronization**: Graal automatically places pipeline barriers and semaphore waits between passes. They are inferred from the accesses made to resources in each pass.
You can now add/remove/reorder passes without having to think about how to update your synchronization code, which makes experimentation with complex rendering pipelines easier.
* **Transient memory allocation**: Graal can reduce GPU memory usage by mapping multiple transient resources to the same memory block, if it detects that the uses of the resources do not overlap within a frame. Doing this kind of optimization by hand is tedious and error-prone when your rendering pipeline is constantly changing.
* **Management of descriptor set layouts**: with procedural macros, Graal can generate code for creating descriptor set layouts and update descriptor sets from a type.


.. it's difficult to provide useful, self-contained and compelling examples because graal-the-library does little by itself:
it doesn't provide anything to load/generate image or mesh data, for example. And setting up a VkPipeline instance is still verbose.
Because of that, examples most likely need to rely on some external libraries.

.. TODO: memory safety is not a goal, but make it "easier to be correct".
.. TODO: full memory safety, can be implemented at a higher level. This library is too low-level and knows too little about your application architecture.


Graph concepts
====================================

Buses
---------------------------

We should avoid cluttered node graphs. As much as possible, the graph should be a simple linear pipeline with clearly defined computation steps,
with branches expressing alternatives instead of parallelism.
For example, let's say we want to apply two different filters on some input image. The two operations are not dependent on each other.
We could represent that as two branches that


Node reference
====================================

Scene filter
---------------------------

The scene filter is in charge of filtering the contents of a scene to be rendered.
Typically, you would create scene filters based on the different kinds of materials in your scene, if you want to render objects with different rendering pipelines.
Scene filter take a camera, used for occlusion culling (optional), and pass down the camera down the line as an associated variable.
Conceptually, the output of a scene filter is a stream of "geometry objects", which translates to vertex buffers and draw calls at a lower level.


Rasterize
---------------------------

The rasterization node


