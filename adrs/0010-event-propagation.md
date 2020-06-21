# 10. Event propagation and focus handling

Date: 2020-06-14

## Status

Living Draft

## Context

This document describes how events should be passed along nodes of the visual tree, and how the visual tree
updates its focused node in response to events.

### Focused node
There is at most one _focused node_ at a time.
The _focused node_, if there's one, captures all _keyboard events_ sent to the visual tree. 
When there's no _focused node_  then _keyboard events_ are not handled. (**TODO** they should be)

### Focus acquisition
A node _acquires focus_ when:
- it receives a _pointer down event_ 
- AND the event handler for the  


When there is a _focused node_, it receives all _keyboard events_ 
Otherwise, _
The _focused node_ receives **directly** all keyboard events

A node becomes the _focused node_ in response to a _pointer down event_ sent to the node, except if the node
explicitly signals that it should not acquire the focus.
