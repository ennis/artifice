# 6. Remove the action type

Date: 2020-06-04

## Status

Draft + Accepted

## Context

The `A` type parameter of the `Widget<A>` represents the type of "actions" that the widget emits in response to events.
For instance, a slider is typically `Widget<f64>`, as it emits the new value every time the slider moves. A TextEdit
might be `Widget<String>`, as it emits the text it contains on every change.
A special widget type, called `Map`, can transform a `Widget<T>` into a `Widget<U>` given a mapping function from T to U.
This is useful to make a list of child widgets emit the same action type, and to compose primitive actions into more complex, 
application-domain [actions].
In the [Elm architecture], "actions" are called "messages".

Actions are emitted by `Visuals` during event propagation (`Visual::event`). 
Those events are sent into an `Rc<dyn ActionSink<A>>`, which:
 - collects the emitted actions into a vector (`ActionCollector`) for later processing
 - OR applies a mapping function defined by a `Map` widget, and forwards the transformed action to 
a parent `ActionSink` (`ActionMapper`)

All `Visuals` that emit actions should hold a `Rc<dyn ActionSink<A>>` member for this purpose. It is not passed as a part of the `Visual`
interface because then the `Visual` trait would need the `A` type parameter, which would complicate the way we store
visuals in the `NodeTree`.

This is a somewhat convoluted architecture, and it's hard to explain the rationale behind `ActionSink<A>` and friends.
Also, a recent design decision has uncovered a problem related to potentially overlapping implementations between `TypedWidget<A>`
and the `Box<dyn Widget<A>>`, made possible by the parametrization on the action type. 

Side note: the issue is that, given `trait Widget<A>` and `trait TypedWidget<A>`, 
`impl<A> Widget<A> for Box<dyn Widget<A>>` and `impl<A, T: TypedWidget<A>> Widget<A> for T` may overlap because according to rustc "a downstream crate may implement
`TypedWidget<LocalType> on Box<dyn Widget<LocalType>>`" (i'm not even sure that the orphan rules currently allow that...). 
Removing the `A` parameter eliminates the issue.

## Proposal

1. Entirely remove the action type parameter `A`, along with `ActionSink` and friends. Visuals don't emit actions anymore.
2. Instead of emitting action objects, visuals should invoke user-provided callbacks on actions (e.g. `on_click(...)`).

This basically removes all action-related complexity from the widgets/visuals, and moves it to the user of the library.
It also makes the interface a bit more familiar.
Users are now free to use the pattern they want to manage actions:
- if the data model is already wrapped into `Rc<RefCell<..>>`, then the user can pass a Rc to the state into the callbacks
and modify the data model from there.   
- if the application has a "message bus" for UI events, then the closure can send events directly to the message bus 
instead of having to go through `ActionSinks` first.
- finally, the user can implement their own `ActionSink`-like object to collect action objects from callbacks, but 
perhaps more directly than constructing actions with `Map`.
    - This moves a bit of the burden to the user, but the simplification of the widget interface is worth it.
    
For now, the closures passed to the callbacks should be `'static`, to avoid infecting the whole `NodeTree` with a lifetime 
parameter. This restriction might be lifted in the future if the lifetimes surrounding the `NodeTree` are clear enough. 
    
## Documenting
This reduces the number of things in the library to document. 
However, tutorial-like documentation should be provided concerning the available patterns for handling widget callbacks 
(especially how to circumvent the lifetime issues produced by the `'static` bound).
    
## Patches
- Remove the `A` type parameter from the `Widget` trait
- Remove `ActionSink`, `ActionCollector`
- Remove the `Map` widget
- ...
- Profit?

[actions]: https://sinusoid.es/lager/actions.html
[Elm architecture]: https://guide.elm-lang.org/architecture/
