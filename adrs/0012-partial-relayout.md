# 12. Partial relayout

Date: 2021-05-01

## Status

Living Draft

## Context

Whenever the application state changes, the whole widget tree is rebuilt from scratch, and the entire layout is recalculated.
This is usually not needed. How to avoid this?

### Rebuilding of the widget tree
It's complicated: currently, the widget tree is built, used to update the node tree, and discarded right after.
It is not supposed to live for long, as it borrows the application state. 
To reuse it, we'd have to store the widget tree somewhere, and thus borrow the application state across GUI updates.

A solution would be to not borrow the application state, but pass it in a type-erased form during recursive layout.
A relayout could then be triggered "in the middle" of the widget tree, as needed.

In this case, the main reason for the split between the widget and node tree becomes invalid (see documentation on `Widget`),
as widgets don't borrow anything.

FIXME: There is another reason for the split: so you can have a widget (logical) tree and a node (concrete) tree that
don't have the same structure.
For instance, a wrapper widget could generate a number of concrete visual nodes to wrap another widget.
For instance, consider DockArea/DockPanel: the DockPanel will sometimes generate a lot of stuff around the wrapped widget.

=> It would be nice to be able to have multiple mutable refs to *disjoint* node subtrees.  

### Partial relayout
Partial relayout could change the measurement of a widget, but it would have no way of signalling that change to parents
since it doesn't know about them (there is no pointer to the parent widget).

A solution would be to have a widget tree that has links to parents. 



## Conclusion
Taking both aspects into account, we can simplify the whole thing by:
- merging the widget and node trees together
- moving layout into the node tree
- widgets perform their own reconciliation, if necessary




