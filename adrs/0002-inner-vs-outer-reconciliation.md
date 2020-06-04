# 2. Interface for reconciliation of widget nodes 

Date: 2020-05-04

## Status

Draft

## Context

### Incremental update
In kyute, GUIs are described declaratively with a tree of `Widgets`. 
When the GUI needs to be updated, the user creates a new widget tree and passes it to kyute. 
Under the hood, this `Widget` tree is used to incrementally update a retained _visual tree_. This visual tree contains
data associated to the visual elements on the screen (layout, input states, etc.), and is invisible to the user.

This incremental update is performed during the recursive layout pass, in `Widget::layout`. During layout, we need to 
match the widget with their associated visual nodes. `Widgets` by themselves don't know anything about their 
associated visual node (by design, because the visual tree is transparent to the user). This matching from `Widget` to node 
is called _reconciliation_ (a term probably coined by React), which matches `Widgets` to visual nodes.

In general, the incremental update process should proceed as follows:
 
- let _cursor_ be a position between nodes in the visual tree, 
- let _widget list_ be a list of widgets to reconcile with the visual tree starting at the location pointed by the _cursor_:                     
- For each _widget_ in the _widget list_:
    - Let _sibling list_ be the list of nodes that come after the _cursor_ at the same tree depth  
    - In the _sibling list_, find a node that matches the following:
        - The type of the _node visual_ is the same as the one created by the _widget_
        - AND the _key_ of the node is the same as the key of the _widget_
    - If a _matching node_ is found, then:
        - Update the _matching node_ from the _widget_
        - Let _current node_ be _matching node_
    - Otherwise:
        - Create and insert a new node at the cursor location. 
        - Let _current node_ to this newly created node
    - XXX: the widget should have layout information here, to determine the _child widget list_
    - If the _widget_ has child widgets, then perform the incremental update process recursively, 
        initializing _cursor_ to point just before the first child of the _current node_, and initializing _widget list_ to the child widgets
    - Move the _cursor_ to just after the _current node_
    
This process is started with _cursor_ pointing just before the root node, and _widget list_ initialized to a list of size
1 containing the root widget returned by the user. 

### Inner vs outer reconciliation

Currently, this algorithm is not implemented explicitly: rather, we expect the individual 
implementations of `Widget::layout` to "do the right thing" and follow this algorithm when called recursively. 
However, in practice, `Widget::layout` is free to update the node tree in completely different ways.

The question is whether letting individual implementations of `Widget::layout` do their own reconciliation is a good 
thing or not. In a way, this goes against the _inversion of control_ principle, and complicates the implementations of `Widget::layout`. 
An alternative approach would be to implement the reconciliation algorithm in one centralized location, 
and modify the interface of `Widget` so that it can provide the necessary data to the algorithm 
(the expected visual type and the key). We call this approach "outer reconciliation", because the reconciliation algorithm
is moved outside of the `Widgets`. In contrast, the current approach which stuffs the algorithm inside `Widget::layout` is called "inner reconciliation".

### Rationale for the current inner reconciliation
The main reason why reconciliation is currently implemented inside `Widget::layout` is type safety: 
within `Widget::layout`, the type of the _node visual_ is known statically, so we have a type-safe way to 
search and return a `Node` with the expected visual type without downcasting.
To have type-safety with an _inversion of control_ approach, we would need a `Widget` trait that looks like this:
```
trait Widget {
    type Visual;
    fn layout(&self, previous_visual: Visual) -> Layout;
}
```
Note the associated type: this caused a lot of ergonomics issues as soon as we need boxed widgets 
(what do we put for `???` in `Vec<Box<Widget<Visual=???>>>` => need BoxedWidget<>?),
and was what led to the current design (without the associated type).

Also, the current `Widget` trait allows a widget to emit more than one node, but the usefulness of that is unclear.


## Decision

We decided to switch to the _inversion of control_ approach, i.e. "outer reconciliation". Type-safety considerations 
prevented us from choosing that option before, but the simplified implementation (and documentation!) of `Widget`
is worth it.


We propose an alternate algorithm for reconciliation:

- let _cursor_ be a position between nodes in the visual tree, 
- let _widget list_ be a list of widgets to reconcile with the visual tree starting at the location pointed by the _cursor_
- let _layout constraints_ be a list of layout constraints for each widget in the _widget list_
- let _measured layouts_ be an empty list               
- For each _widget_ and _constraint_ pair in the _widget list_ and _layout constraints_:
    - Let _sibling list_ be the list of nodes that come after the _cursor_ at the same tree depth  
    - In the _sibling list_, find a node that matches the following:
        - The type of the _node visual_ is the same as the one created by the _widget_
        - AND the _key_ of the node is the same as the key of the _widget_
    - If a _matching node_ is found, then:
        - Update the _matching node_ from the _widget_
        - Let _current node_ be _matching node_
    - Otherwise:
        - Create and insert a new node at the cursor location. 
        - Let _current node_ be this newly created node
    - (`Widget::layout`) invoke the widget layout phase 1 procedure (measurement):
        - (`Widget::layout`) if the widget has child widgets, determine their layout constraints, and perform the incremental update process recursively with:
            - _cursor_ pointing just before the first child of the _current node_
            - _widget list_ initialized to the child widgets
            - _layout constraints_ initialized to the determined layout constraints
        - (`Widget::layout`) return measured layout of the widget
    - add the return value of the widget layout procedure to the list of _measured layouts_
    - Move the _cursor_ to just after the _current node_
- return _measured layouts_


This comes with a simplified `Widget` trait that looks like this:

```rust
trait Widget {
    fn key(&self) -> Option<u64> { None }
    fn node_type(&self) -> Option<TypeId> { None }
    // phase 1 layout    
    fn layout(self, ctx: &mut LayoutCtx, constraints: &BoxConstraints, prev_node: Option<Box<dyn Node>>) -> (Box<dyn Node>, Layout);
}
```

Also, add the following methods to `LayoutCtx`:
```rust
impl LayoutCtx {
    pub fn layout_child(&mut self, widget: impl Widget, constraint: BoxConstraints) -> Layout 
    {
        // ...
    }
    
    pub fn layout_children(&mut self, widgets: Vec<Box<Widget>>, constraints: Vec<BoxConstraints>) -> Vec<Layout>
    {
        // ...
    } 
}
```

`Widget::layout` has now less responsibilities: 
    - if the widget has no children, then it simply measures itself and sets its size
    - otherwise:
        - it should determine the constraints for all its children and put them in a vec
        - call `LayoutCtx::layout_child` on each children, passing the computed constraints.
As a convenience, `LayoutCtx` also provides a `layout_children` if the widget already has a vec of child widgets. If the vec
is not as big as the list of widgets, then the last constraint in the vec is used for the remaining widgets.

Note that `Widget::layout` is only concerned with the _measurement_, and not the _placement_ of its children. 
Placement of children is now deferred to the `Visual`.
In theory, all the layout (measurement+placement) could be done in the widget, but this adds responsibilities to the
widget, and it needs another API in `LayoutCtx` to place children.
The measurement must be done in `Widget`, though, because it might produce a different number of child widgets to fill
the available size.

The previous `layout` had `nodes` and `cursor`, and it's unclear what it should do with it. It also mixed layout computation
with reconciliation.
Now the reconciliation is done inside `LayoutCtx`, and the layout proper is done within `Widget`.
 
## Consequences
Greatly simplified implementation of `Widget::layout` in many cases.

