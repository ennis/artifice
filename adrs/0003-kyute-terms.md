# 3. Glossary of terms in kyute 

Date: 2020-05-22

## Status

Draft

## Context

This document is a glossary of the terms used to describe the parts of kyute. It is there to provide a reference and 
clear up ambiguities about the names used in the code base.

- _Widget_: it's a widget, like in other frameworks. A visual element of the user interface that sometimes displays 
 data to the user and sometimes responds to input events. In the context of kyute, _widgets_ are represented by types
 that implement the `Widget` trait, and are composed into trees.
 
- _Widget tree_: the tree of widgets that describes the GUI. It loosely corresponds to the "View" in the 
"Unidirectional-data flow" architecture (see https://sinusoid.es/lager/architecture.html). Every time the GUI should 
update, a new widget tree is created from scratch that represents the current state of the application. This tree is 
then "executed" (`Widget::layout` is called recursively), which updates the node tree.

- _Node tree_: the retained visual tree of the GUI. It contains the state of GUI elements that we want to keep track of
across widget updates. You can think of it as the counterpart of the DOM in frameworks like react.

- _Visual_: the visual defines the behavior of a node: how it is drawn to the screen, and how it responds to input events.

- _Platform state_: 

- _Device pixel_: a pixel on the render device
- _Device-independent pixel (DIP)_ -> all sizes in kyute measured in DIPs
- _DIP scaling factor_, or just _scaling factor_ -> how many device pixels are in a DIP
