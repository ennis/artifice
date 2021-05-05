# 1. Design of the visual tree of kyute

Date: 2020-05-04

## Status

Accepted

## Context

`kyute` is the user interface library used in artifice, currently in development. For various reasons outside the scope of this document,
we decided to use an architecture similar to Elm (https://guide.elm-lang.org/architecture/), in which the UI emits
_actions_, which are handled by the application, which in response updates its internal data model and produces a new 
_view_ which is the visual representation of the data model shown on the screen. From the user point of view, a _view_ is a tree of _widgets_. 
A new view is generated from scratch on each update. 
To avoid polluting the application data model with view-specific state (such as the position of scroll bars, or the 
state of a collapsible panel), widgets can keep internal state across updates. 
To relieve the user of managing this state, we decided (TODO ADR) to split the widget tree in two parts: the _widget tree_,
composed of _widgets_ and the _visual tree_, composed of _visual nodes_ (or just _nodes_).

Widgets, which implement the `Widget` trait, are composed by the user in a tree that describes the user interface.
This _widget tree_ is passed to kyute, which runs the layout pass on it by calling `Widget::layout`. 
This pass updates the _visual tree_ to match the new _widget tree_ by modifying existing nodes or creating/removing 
_visual nodes_ (or just _nodes_) if necessary. We call this process "reconciliation", similarly to React (https://reactjs.org/docs/reconciliation.html). 
The _widget tree_ is re-created every time an update is needed so it's short-lived. The visual tree however is long-lived and incrementally modified.

_Visual nodes_ represents a visual element on the screen: usually a widget produces one visual node, 
but sometimes we reuse visuals between widgets that have the same draw and event behavior but different layouts.
 some widgets can produce a node that contains other nodes as well (it happens when we want to reuse visuals between widgets).
Visual nodes know how to draw themselves on the screen and how they should respond to input events.
Since this varies between different types of nodes, this behavior is abstracted behind a `Visual` trait (TODO ADR), and stored in a `Box<dyn Visual>` field in the nodes.
Usually, a widget has one associated `Visual` type, but for some widgets that only differ by their layout we reuse the same
`Visual` type.

Calculated layout data is stored directly in the node (and not in the `Visual` object) since it's common to all nodes. 

In short, a visual node is something like this:
```
struct Node {
    layout: Layout,     // calculated layout
    visual: Box<dyn Visual> // draw and event behavior
}
```
Not pictured: the tree infrastructure (parent node, children, siblings).
There was some uncertainty as to how best implement this. 
For reference, the tree should have the following features:

1. Easy to manipulate:
    The tree should be easy to manipulate: add/remove/insert nodes in the middle of the tree.

2. Easy to traverse:
    The tree should be easy to traverse, because the reconciliation algorithm sometimes needs to search the tree.

3. Addressable nodes:
    To implement various GUI behaviors easily (TODO ADR), we decided that we should have some way to refer to the widgets:
    a pointer, an ID, a name, etc.


We considered and experimented with different options:
1. The `Visual` owns child nodes. This worked for a while and had the advantage that a visual could control the type of 
child nodes that it accepts. However it's very difficult to reach a node stored within several layers of `Visuals`:
    - We can't just keep a borrow of a node nested in visuals: the borrow checker would lock the entire tree.
    - We could store nodes as `Rc<RefCell<Node>>`, and keep references to nested nodes with `Weak<RefCell<Node>>`,
      but that's not fun, and borrow checking now happens at runtime. 
    - We could assign an ID to each node, but then to reach it (i.e. get a mut ref to it) we would have to search the 
    tree starting from the root every time we want to access a node by ID.
        - the `druid` library does that, and uses bloom filters to optimize the search. Keeping the bloom filters up to date
          across insertion and deletion of widgets seems somewhat hard. 
          They don't seem to insert/remove widgets often though, because of other architecture decisions.
2. Use an existing tree structure that allows us to access nodes by ID.
    - indextree
    - id-tree
Some of those have been used before to represent DOM-like trees.
    

## Decision
We decided to use an indextree with generational IDs (https://crates.io/crates/generational-indextree). We can refer to nodes
by ID, and IDs can be checked for validity in case the pointed-to node has been deleted. The "owned-children" option was
implemented before but it was becoming too impractical to build typical GUI patterns with it (identifying the focused widget, 
the widget that's grabbing the mouse, moving the focus around, etc.).


## Consequences
- In some implementations of `Widget::layout`, there were mut borrows of a child node and a parent node at the same time.
This was possible with owned-nodes because borrowck could prove that the borrows were disjoints (different fields of the parent visual).
This is harder to do with the current indextree-based implementation because all nodes are stored in a big array and 
there are no "split-borrows" like in Vec. The code has been refactored to avoid mut-borrowing multiple nodes at once,
with minimal readability impact.

- `indextree` stores the nodes in linked lists, for optimized insertions and removals. 
Each node has thus an memory overhead of `5*sizeof(NodeId) (parent,prev_sibling,next_sibling,first_child,last_child) = 40 bytes`,
that was not present in the previous version. We consider this overhead to be negligible.

- The number of dynamic allocations should not have changed much; it's unclear which approach is the most performant,
but impact of the change is seems to be low anyway.   