# 4. Rust module organization

Date: 2020-05-24

## Status

Draft

## Context

Sometimes it's unclear how to split rust code in multiple modules, especially for . This document 
tries to establish guidelines to do so.

### Purpose
A good module organization does the following:
- makes it easier to identify the different aspects of the program
- reduces the conflicts between people who work on different features 
- facilitates the parallelization of feature development 

Corollary: 
a good module organization should allow a contributor to touch as few files as possible when
working on one specific aspect or feature of the problem.

### Guidelines:

0. Do not let module organization concerns get in the way of code clarity.
Moving items in separate modules should not add more code.

0. The module structure exposed to the user may be different than the internal module structure: you may not want to 
expose how you organized the code internally to the user, but instead show them a flatter namespace.
This can happen if there are few, but interconnected, types with a lot of logic: you may choose to split the logic in 
different submodules, and not expose that choice to the user.

0. Regroup items that depend on each other's internals in the same module.
 
0. Sometimes you should _expose_ submodules if there's a set of types that the user may not want directly in
scope (e.g. because there are a lot of types and their names may conflict with others in scope).

0. Regroup items that are usually used together in the same module

0. Try to identify self-contained "concerns" or "concepts" or "aspects": put them in the same (internal) module.
E.g. in kyute, put everything used for _reconciliation_ in the same module. Same for _painting_, _layout_, etc.


In short, split in modules first for the contributors, and then for the user of the library. In general, 
you can get away with a flat namespace, but if you can identify sets of types that obviously belong together without a 
lot of dependencies on the outside, then make a module for them.

## In kyute
It might be tempting to split `Widget`, `Visual` and `NodeTree` into their own separate submodules and expose that to the
user. However, those modules will have only one or two type or traits and will rarely stand on their own (`Widget` is used
by the `NodeTree`, same for `Visual`). So all three should probably be in the same root namespace, to clearly reflect 
the fact that they are interconnected.

Example internal module hierarchy:
- `node`: contains the implementation of the NodeTree
    - `node::event`: event propagation (defines `EventCtx`)
    - `node::reconciliation`: reconciliation code
    - `node::layout`: code for the layout pass (defines `LayoutCtx`) 
    - `node::paint`: painting logic (defines `PaintCtx`)
- `visual`: contains the definition of the `Visual` trait, along with default visual
- `widget`: contains the definition of the `Widget` trait, also: root module for common widgets 

External (exposed) module hierarchy:
- root: `Widget`, `Visual`, `NodeTree`, `LayoutCtx`, `EventCtx`, `PaintCtx`
- `layout`: `Layout`, `BoxConstraints`, `Alignment`
    -> most types here are standalone, and don't depend on other modules to perform their functionality
- `visual`: `Visual` trait, and root module for common visuals
- ... and others ...

