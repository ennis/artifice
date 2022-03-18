==============================================
MLR notes
==============================================

MLR initially stood for mid-level renderer as it was supposed to stand between the high-level renderer and graal.
But the current goals of MLR, which is basically creating a way to express shader interfaces using rust syntax and proc derives is useless for the needs of HLR. So instead just make MLR be the actual high-level renderer.


Concepts
-------------------------------------

MLR is intended to receive data from the application with minimal processing and render it. What concepts should it handle?

* shader graphs?
* image buses? (i.e. "fat" images with many channels)
* *NOT* a general evaluation engine
* abstract bindings (descriptor stuff)

What form takes the input to MLR? Is it retained (modified incrementally) or rebuilt every frame?
- Rebuilt from scratch on every evaluation: we must have a complex caching/hashing mechanism to avoid rebuilding costly objects (such as shaders). I.e. must hash everything, including shader source code, pipeline config, and possibly images.
- Rebuilt from scratch, but can reuse elements that do not change: immutable values
    - preferred approach, worked quite well with kyute

What objects?
* built shader networks

Types of nodes:
* draw pass


Shader snippets
-------------------------------------

