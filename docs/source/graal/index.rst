Graal
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


Passes and frames
====================================


Passes
---------------------------

Commands sent to the GPU are regrouped in *passes*. 
When building a pass, you also declare the resources that the commands of this pass will access.

For instance, a pass that renders a mesh needs to *read* the vertex/index buffers, *write* to the framebuffer images, and potentially *read* from textures. You need to declare all of those accesses in the pass, like so:
``
TODO
``

With this information, Graal automatically infers the necessary pipeline and memory barriers between each pass.



Frames
---------------------------

Passes are themselves regrouped in *frames*. A frame is delimited by calls to ``Context::begin_frame()`` and ``Context::end_frame()``.






* User documentation
	* Quick look



* Overview:
	* Context
		* Resources
	* Frames 
		* Frames, Passes
		* Resource accesses
	* Vulkan utilities
		* Derived descriptor set layouts
		* Derived vertex attribute description
		* Derived render passes (fragment output interfaces)


- Developer documentation
	- Resources
		- Tracking information
	- Frame construction
		- Inference of execution and memory dependencies  

