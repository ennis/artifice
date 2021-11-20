# 14. Kyute revisited

Date: 2021-06-25

## Status

Draft

## Context

Druid works quite well, but integrating 3D into that seems cumbersome, mostly because of the required changes in 
druid-shell (namely, exposing the D3D & D2D contexts to the client). Plus, there's a lot of things done in kyute already 
(text editing works!).


## The big picture

A GUI application is composed of two elements: the data, and the GUI. 
There's some kind of function `F(Data) -> GUI` that produces the GUI from the data. 
The data changes over time, but re-evaluating the function and re-building the result may be costly, so we want to do
things *incrementally*: i.e. from a small change in the data, make a small change to the previously returned GUI to 
make it match the current data, instead of rebuilding it from scratch. 

It is also important to not destroy parts of the GUI that don't change because some GUI elements have *internal state* 
that we want to keep as much as possible across data updates 
(e.g. the position of a scrollbar, the currently selected tab in a tab view, whether an accordion is collapsed or not...).

There are several ways to perform incremental updates:
- don't: rebuild everything from scratch every time 
    - examples: imgui
- events-based: whenever a part of the data changes, an event is emitted, which is handled by parts of the UI that depend
  on the data. Those parts then update themselves with the new data.
    - examples: Qt, and a lot more
- traversal: the GUI holds the previous version of the data that it depends on. Whenever the data changes, the GUI and the data are
  traversed simultaneously and the widgets update themselves if their data is out of date.
    - This supposes that each piece of data is cheap to copy and compare (value types). Having data like this has other advantages, however.
    - examples: druid
- reconciliation: same as the traversal, but instead of comparing against the previous data, compare against the created widget.
    - examples: react
    
The druid approach seems to work rather well. The restrictions on the data model may seem harsh, but they bring advantages
like easy undo/redo.

