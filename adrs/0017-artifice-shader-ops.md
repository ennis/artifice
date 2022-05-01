# 17. Artifice - shader nodes

Date: 2022-04-26

## Status

Draft

## Context

Artifice is a tool for quickly prototyping real-time rendering techniques. It is node-based, although the UI exposed to 
the user may take a more convenient form than node graphs, depending on the context. 
This document proposes a design for _shader nodes_ in artifice, that are connected together to form shader graphs
that describe a graphics or compute pipeline.

But first, some reminders (some of which were not formalized in ADRs yet):
- "nodes" means the data model objects that contain attributes, connections, etc. Perhaps confusingly, _shader nodes_ are a different concept that
  only make sense during evaluation.
- The behavior of nodes are defined by _operators_. Not all nodes have an associated operator, as there are nodes whose sole
  purpose is to contain data.
- Operators implement certain interfaces (traits), specific to some _context_. The implementation performs some context-specific work, given a reference to the node containing the actual data. For instance:
  - Operators that run under the _imaging_ context produce images, given a region of interest (RoI) and current time. They implement the `ImagingOperator` (TODO) trait.
  - Operators running under the _shader_ context produce graphics or compute pipelines that can then be run on the GPU. They implement the `ShaderOperator` (TODO) trait.
  - Finally, there is the _general_ context, for nodes that produce simple values given the current time.
- Each context has their own type of arguments: the RoI for imaging context, the current time for the general, etc.
- Contexts can _inherit_ from others, meaning that a context can expect the arguments from another context to be present (e.g. imaging expects the general args to be there). 

- The operator contexts are concrete objects, like `OpGeneralCtx`, `OpShaderCtx`, `OpImagingCtx`, that have **their own traversal policies**. 
  In a way, they describe a computation on a node graph. 
- Contexts have associated caches that they can query by type (probably stored in a typemap).

- to evaluate something, you need to know what kind of value it is, and what context it needs. Then, you can spin a context of the correct type to evaluate it.
  The context fetches a reference to its cache from somewhere, tries to look up an existing value. If there's one, return it, otherwise call the operator to perform the evaluation.

- Looking up values from the cache: each evaluation is mapped to an ID in a slotmap.
- Evaluations are looked up by hash (u128), which is composed of the model path to the object and the evaluation context hash
  - `value_ids: HashMap<ModelPath,EvalId>`
  - `values: SlotMap<EvalId, Value>`
- values _can_ be stored in the main eval cache:
  - in struct `Value`: `cached: RefCell<Option<Arc<dyn Any>>>`
  - but it can also be stored in a specialized cache
  

~~- You can pass any number of contexts to the evaluation process. They can be queried by type. (probably stored in a typemap)~~


## Terminology
- Storage format: how the data is stored on-disk
- Schema: description of the structure of the data

