# 7. Pixel snapping

Date: 2020-06-05

## Status

Draft

## Context

The layout pass can produce sizes and offsets that don't line up precisely with physical pixels on the screen,
resulting in blurry borders ("lines between two pixels").

This document examines different methods to do _pixel snapping_, which consists in rounding the sizes and offsets so that they
line up with physical pixels.

### Nomenclature
- how do we call things?
- device pixel: a pixel on the render device
- device-independent pixel (DIP) -> all sizes in kyute measured in DIPs
- DIP scaling factor, or just "scaling factor" -> how many pixels

### Automatic snapping on paint
Before calling `Visual::paint`, snap the current transform to the nearest (?) physical pixel. 
Also round up `bounds.size` to the next physical pixel boundary.
This means that `bounds.size` is not guaranteed to be exactly equal to the value returned in `Widget::layout` (is that a problem?) 

### Expose DPI scale factor during layout
This way, widgets can round up their desired size to the next physical pixel boundary manually.

## Considered alternatives
- Use device pixels during layout and paint
    - No snapping problem, but the it now relies on the widget layout implementation to size themselves correctly
    

## Decision
- Add the following methods to `LayoutCtx`:
```
/// Returns the DPI scale factor (device DPI / 96.0) of the screen
fn scale_factor() -> f64;
/// Sets a flag on the current node that indicates that the rendering pass should snap 
/// the window position and the size of the node to device pixel boundaries   
fn snap_to_device_pixel();
```
- Add the following extension method to `Point`:
```
fn snap_to_device_pixel(&self, scale_factor: f64);
```
- Add the following extension method to `Bounds`:
```
/// Rounds up width and height to the next highest multiple of the device pixel size.
fn round_up_to_device_pixel(&self, scale_factor: f64);
```