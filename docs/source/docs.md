
# Passes and frames

Commands sent to the GPU are regrouped in *passes*. 
When building a pass, you also declare the resources that the commands of this pass will access.

For instance, a pass that renders a mesh needs to *read* the vertex/index buffers, *write* to the framebuffer images, and potentially *read* from textures. You need to declare all of those accesses in the pass, like so:
```
TODO
```

 With this information