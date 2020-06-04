# 4. Kyute themes

Date: 2020-05-30

## Status

Draft

## Context

This ADR contains thoughts on themeing widgets.
 
Right now, in kyute, we have a `Theme` struct that handles the rendering and measurement of "frames": 
primitive visual elements (usually boxes with specific borders, padding and backgrounds)
that make up widgets on the screen. This struct is passed to `Widgets` during the layout pass,
and `Visuals` during the painting pass. 

Measurement of frames is done in two parts, much like layout: first, given sizing constraints for the frame (_frame constraints_), 
you ask the theme for the sizing constraints for the content (_content constraints_). Then, you run the layout
on the content using the _content constraints_, producing the _content size_ and then, given the _content size_,
ask the theme to measure the frame and place the _content box_ inside it, producing a _size_ and an _offset_ for the 
content inside the frame. This is done in the layout pass (`Widget::layout`).

Drawing a frame is simpler, you just pass the paint context and the bounds of the frame to render. This is done in 
`Visual::paint`. 

There are special methods to get the measure of some items such as sliders, and a method to render a text edit box.

The issue with the current API is that the user can't extend the interface of themes to handle their own custom frames.
We propose a redesign of `Theme` so that a user could also use it to style their custom frames.

- Proposal 1: downcasting.
We pass a trait object `Theme` to layout and paint, but this object is "downcastable" to other interfaces for custom
styling (i.e. base `dyn Theme` object, from which we can get a `dyn ThemeForSomeCustomWidgets`).
This requires querying a trait object whether it implements a given trait or not.
With this design, fallback (if the theme doesn't handle the custom widgets) must be handled by the `Widget` and `Visual`. 
This adds some checks in the widget part that pollute the code a little bit. Also, the theme itself can't specify defaults.

- Proposal 2: generic frames
We take inspiration from Qt (QStyle) and propose an open-ended interface, where frame types and metrics are identified by keys 
(strings, integers, something else?).
We have identifiers for _frames_ (_Primitives_ in Qt parlance), _subelements_ of complex widgets (e.g. the 
track line of a slider widget).
Some frames take additional parameters for measurement and drawing, which are passed as `&dyn Any` (Qt: QStyleOption). 
The expected type is not statically checked, but must be documented.
Custom themes can "inherit" from a base theme by simply delegating to a base theme instance when they don't recognize 
the key (Qt: QProxyStyle).

Examples of frames:
- button ("kyute:button")
- text edit ("kyute)
- slider
- ...

Examples of subelements:
- slider track (slider)
- slider knob (slider)


 
```

const BUTTON_FRAME = Primitive("kyute:buttonFrame");    // Button
const SLIDER = Primitive("kyute:slider");               // Slider and knob
const TEXT_EDIT_FRAME = Primitive("kyute:textEditFrame");   // Text edit border and background

const SLIDER_TRACK = SubElement("kyute:sliderTrack");
const SLIDER_KNOB = SubElement("kyute:sliderKnob");

trait Theme {
    
}
```
 
 
Traversal: Root(kyute:root) . Flex(kyute:flex) . Flex(kyute:flex) . Button(kyute:button) . Text(kyute:text+kyute:label)

- Root
- Button
- Text
- Panel
- Flex
- Text
    
Several rules apply to the text:

kyute:label
kyute:text
kyute:button
kyute:flex
kyute:root

All values of properties are resolved during layout (this is because a change to a property can influence the layout).
Some properties may change according to state (hovered, active, etc.): in this case, there are multiple strategies:
- return not a concrete T but a function fn(State)->T (`Property<T>`)
    - `Property<T> encapsulates the value of a style property that may vary according to state and time (animation)`
- change the API to `resolve(selector, state)->T`
- store the resolved style stack in the node (`Vec<Rc<Rule>>`) **
    - LayoutCtx and PaintCtx have functions to query the final value of a property.
    - Since PaintCtx has access to the current animation state and state of the visual, it does not need any other parameter
      to resolve a state. 

### Animations?
Some properties can be animated. However, animations should not trigger a relayout.
Layout-related properties thus can't be animated (and cannot have different states either).

## Changes:
Where T can be:
- A `Copy` type (`f64`, `Color`, `SideOffsets`, ...)
- `dyn Trait`

Representing the state of a visual.

- Proposal 4: swiftui environment
-> pass values to children
-> no classes
-> can contain anything, even types that modify the emitted widgets during layout (e.g. emitted widgets)

```
trait ButtonStyle {
    fn make(&self) -> Widget<>
}
```

I'd like to associate defaults to keys, but some values can't be const (e.g. Color, or anything that needs to be boxed before):
```
pub struct Key<T> {
    name: &str,
    default: T
}
pub const FONT_NAME: Key<str> = Key { name: "fontName", default: ??? };
```

Solutions:
- don't address the issue: require careful initialization of environments with all the keys in a global function
- when defining a key, also define a default value
    - can't do it due to const limitations with "const-value" keys, but possible with "keys-as-types" (`default() method`)


Two conflicting features:
- default values for every key
- returning types by reference instead of by value to accomodate unsized types (dyn Trait, str)

This means that the `default()` for the key must return a `'static` reference. I.e. it must have a `static T` inside of 
the function, which needs to be able to build a `T` in const contexts (not always the case), or use `lazy_static`
(impractical, relies on an external macro).

```
pub trait Key {
    type Value: EnvValue + ?Sized;
    fn default() -> Self::Value {}
}
```

