
## veda address/index simplification
- no lenses?
    - only the key path value + types
- Address ~= swift KeyPath 
- Addressable trait
- Address<Root,Leaf>: PartialAddress<Root>
    - <State as Addressable>::get(&self, PartialAddress<Root>) -> &dyn Addressable
    - <State as Addressable>::get(&self, Address<Root, Leaf>) -> &Leaf
    - `Address<Root,Leaf>` **always** convertible to `PartialAddress<Root>` 
- Similar to current lenses, but less syntactical overhead
- Difference: to access an element of a struct, must match the PartialAddress in Addressable::get
    - with lenses, it's simply executing the trait method
        - in practical terms, the offset to the field is encoded in the closure
    - is this a big problem? probably not
- Pros:
    - simplification (only addresses, no more lenses)
- Cons:
    - switch lookup to get the reference to the data
        - in swift, directly contains offsets to the fields (compiler magic)
        
## address simplification
- return to the untyped addresses made of chains of u64 ids
- support lenses only on ordered containers and associative containers with Identifiable elements
- <T as Identifiable>::Key should be convertible to u64
    - no string identification
- Precedent: https://github.com/plausiblelabs/lens-rs
- Q: why?
    - do we gain anything compared to the current address impl?
        - maybe less generated code, but that's about it
        

## Lensless design
- typeless
- no lenses
- trait Addressable: with path, returns another &Addressable
- path is optionally-typed 
- key path can be index or string (what about keys in maps?)
- issue: all modifications must happen via path, dynamic type checking

```rust
trait Addressable: Any {
    fn get(&self, path: String) -> Option<&Addressable>;
}
``` 

- the real issue is tracking changes
- two options:
    - automatic (veda): data as a database with triggers
        - must wrap every modification to the data
        - no control over the modification of the data: the structure is exposed
    - manual: emit events via message bus and custom interface
        - control over the update process, but must write the topic interface manually
        - UI update code must also be written manually (no concept of "property" or "binding")
        
- Alternative:
    - define data model objects as opaque objects with "properties"
    - UI queries those properties
    - update?
        - example: modification of a property deep within the data model
        - must emit event, but where? 
        
- Proposal: decouple data access from update description
    - each data model type has an associated "change" type that describe what has changed, but it's not 
      directly linked to the members (or automatically derived, for that matter)
    - can control precisely how a data model type communicates changes
    - no need for lenses anymore
    - in views: guarded by change
    
```
struct Input {
    name: String,
}

enum InputChange {
    All,
    Name,
}

struct Node {
    name: String,
    inputs: Vec<Input>,
}

enum NodeChange {
    Name,
    Inputs(CollectionChanges),
    Input(usize, InputChange),
    Outputs(CollectionChanges),
    Output(usize, OutputChange)
}
```
Or, if precise changes are not needed:

```
struct Input { ... }
struct Node {}
enum NodeChange {
    Name,
    Inputs,
    Outputs,
}
```

- Why does this have to be a value?
    - For composability (and decomposability)

```

```

## Options for "property bindings"

```
View {
    text = <expression>,
    contents = <collection-binding>
}
```

- things to consider
    - sorting of data elements according to a predicate
    - boilerplate code
    - do we want a generic treatment of properties?
    - type-erased views/lenses (access data by string keys / type-erased hashes)
        - external editor
        - huge simplification
        - live-reload?
        
- Overall goals:
    - Reduce code duplication
    - Simple widget implementation
    - Prefer statically-generated code
    - Reduce usage of boxed trait objects
    - Allow reusable components 
        - i.e. simple to implement a non-primitive view
    
- Consider things from the point of view of the implementor and from the user
    - From the POV of the View:
        - want properties that are:
            - scalars
            - lists of things in a specific order, accessible by index
            - lists of views
        - want a clean, easy syntax for specifying the value of those properties in the constructor (or with named accessors)
    - Two main approaches here:
        - properties are trait objects that compute a value from the ambient state (S), possibly with hooks
            - Simpler to implement, more straightforward?
            - Property bindings are "reified"
            - basically a lens, but more generic (produces a value instead of returning a reference)
        - properties are get/set pairs (or equivalent) that are set through external code in the "bindings" wrapper
            - very easy to implement for the View code (get/set pairs on views)
            - state management can be done entirely outside of the widget
                - testing revisions, etc.
                - "everytime the state changes, run this code on the inner view"
                - there's no need for "properties", except as syntax sugar
                - easy to introduce intermediate variables (or even local state!)
                    - with the binding approach, how to introduce a subexpression shared between multiple properties?
            - need to rely on external code for clean syntax
            - for collections: return a mut ref to a collection, then call "update"
                - issue: View needs to track changes
    - the trait object approach seems closer to the existing code base
        - although it is heavy in terms of boxed trait objects
    - with option two, what's the API for VBox (to add views, etc)?
        - views().update(...)
        - views().insert(...)
        - views().splice(...)
        - views().sort(...)
        - views().sync_with(revision, mapper)
            - if modification of a part of the list, how to decide whether to re-create the view or to update the existing one?
                - always update the existing one
        - `views()` returns proxy object that mut-borrows self, watches modifications
        
- Option A: binding approach
    - the internal state of the widget is automatically bound to a function of the ambient state of the view
    - (-) more restricted
    - (+) state consistency (binding) is encapsulated in reusable types
    - (-) allocation overhead
        - say, list of 1000 items with 3 widgets, each having 6 properties:
            - 1000 * 3 * 6 = ~18000 small allocations
        - high pressure on allocator 
            - maybe, can't really know if it's a problem or not
- Option B: stateful approach
    - can access and set the internal state of the widgets
    - (+) more control
    - (-) user code (or codegen) must ensure consistency

Alternatively:
- Option A: code in property objects, inside view
- Option B: code outside of view

Alternatively:
- Option A: update in a method, local to the view
- Option B: non-local update method (same function for a view and the children)
    - issue: since it's non-local, need knowledge of internal structure
    - in other frameworks/languages, would simply keep a reference to the widget down the tree
        - does not work / hard to do in rust because of strict ownership
            - need Rc
    - can use lenses for that
        - when adding an item to a polymorphic collection (dyn Trait), also return a lens that returns the item with 
            the correct derived type.
        - this assumes that the view structure hasn't changed in the meantime 
    - the problem here is type-erasure
        - once we add stuff to a vbox, the structure is lost (opacified behind dyn View)
        - key insight: a statically-known vbox of views should have a statically-known type 
            - swiftUI does this
        - VBox<(T1,T2,T3,T4)> where T1, T2, T3, T4: View
            - variadic templates
        - then, access via element accessors
            - vbox.content().0.text().update(...)
        - this means that the whole tree in the macro is encoded in the type

Alternatively:
- Option A: State type parameter
- Option B: un-parameterized widgets

- Existing frameworks:
    - druid: binding (ambient state only)
    - WPF: DependencyProperty (somewhat similar to bindings)
    - javafx: properties (bindings, two-way)
    - swiftui: Option B?
    - react: reconciliation
    - flutter: rebuild tree + reconciliation
    - imgui: full rebuild
    - Qt: Option B (but low-level)
    - reactive: binding objects

Challenge with option B: set properties of elements inside (type-erased) containers:
```
VBox {
    HBox {
        // 1
        Border {
            Label(.text = .name)
        }

        // 2
        VBox {
            contents = ForEach in .outputs { 
                Label(.text = .name)
            }    
        }
    }
}

// 1
update(Revision<S>) {
    rev.focus(name) |rev| {
        // need to synthesize this access path
        vbox
            .contents()
            .get_mut(0)
            .downcast_mut::<HBox>()
            .unwrap()
            .contents()
            .get_mut(0)
            .downcast_mut::<Label>()
            .unwrap()
            .text()
            .update(rev)        // wow

        // ... or save and use a lens
        LabelLens::get_mut(self.vbox).update(rev)
        // ... or propagate the change down the line, to the label
    }
}

// 2
rev.focus(outputs) {
    vbox.get::<HBox>(0).get::<VBox>(1).contents().update_with(rev, S->V) 
}

```

with Option A, it's easier: the state "trickles down" the widget tree naturally until it reaches the label.
You can't "look inside" the contents of a view from a higher-level widget.

### Problem with property bindings:
- One property may update with a revision, but also need the value of **other properties** in update.
    - must be different subtrees
- Otherwise, just take a single parameter

### Option A: `View` has binding trait objects that compute the value of the field from the ambient state `S`  
- The 'purest' approach (similar to what we do now)
- No need for the `Binding<...>` wrapper type 

### Option B: `View` has get/set pair for each property
- get/set_text
- get/set_contents
- Issue: collection properties
    - set_contents takes a parameter that describes a change to a collection
        - replacement
        - splice
        - relayout (sorting)
- some views may want their own sorting mechanism
    - e.g. display items, but sort the view according to different criteria
    
### Option C: `View` has methods that return `impl Property`, mut-borrows View


### Issue: can't trickle a computed expression down the line
e.g.
```
VBox {
    Label { 
        text = .name.append("...")
    }
}
```
-> actually possible, just materialize the expr in the update method.
    -> also cache and watch changes


### What is a view:
- inner structure (views), hidden from the user in most cases
- update function that incrementally updates the inner structure given a revision
- the ambient state, a type parameter that indicates "what the view is viewing"

```
// Ambient state: Node
// Has Node.outputs: Vec<Output>
VBox {      
    contents = in .outputs {
        // Ambient state: Output 
        //  -> contents must be Vec<View<Output>>, 
        // if contents = Vec<Box<dyn View>>
        //  -> on update, must downcast to the actual type of the view
        Label(.name)
        Checkbox {
            checked = .enabled
            label = Label(.name)
        }
    }
}
```

```rust 
fn update(&mut self, rev: Revision<Node>) {
    if let Some(rev) = rev.focus(Node::outputs) {
        self.contents().update_with(rev, |output| {
            VBox {
                Label,
                Checkbox
            }
        }, |view, rev| {
            // view: &mut T where T is the return type of the closure above 
            // also have access to the parent revision (of type Node), which is nice
        })
    }
}
```

## artifice

Things to port from autograph:
- Uniform layout checks 
- shader interface checks

## Use native text widgets for custom UI?
- don't bother, no big application uses the win32 controls anyway
    - firefox uses gecko, which uses custom rendering
    - same with java UI toolkits
    - Qt does its own rendering

## GLTF
- maybe we don't need to convert GLTF to an internal representation?
    - the renderer just consumes GLTF directly?
    - no: not meant as an in-memory representation

## Components of the data model:
- context (outside of the data model)
- windows (runtime model)
- scenes (document model / scenes)
    - geometries
        - morph target
            - GPU buffer(s)
    - materials
        - standard viewport material
            - color
            - specularity
            - associated shaders
        - others? 
    - post-effects
        - shaders
    - animations
        - GPU buffers
    - object
        - geometry reference 
        - morph weights
        - animation
- renderers (runtime model)
    - scene ref + camera
    - renders a scene to a window
- open documents (document model)
    - scenes + camera + renderer configs + undo list
    
## Next steps to open a window
- Use glutin + imgui
    - OK
- Use druid-shell + glutin (context creation) + imgui
    - OK, some redundant code, but we also get:
        - main menu
        - context menus
        - 2D drawing library (piet, via direct2d)
    - we won't use the additional features immediately, but might in the future
    - note: it's a pain in the ass to do anything with the platform-specific handle...
    - not polished...
- kyute + Qt OpenGL
    
## Should we continue kyute?
Is it worth binding all of Qt when we need to fight around the way Qt is designed (signals/slots, event loops, etc.)?
A pure-rust equivalent would be preferable in the long term, but cannot be a single-person effort.
`druid` is on the way, but unsure about the way they chose for representing state (they want immutable data structures for 
quick diffs). 


## artifice windows
- There is a table of open windows, identified by ID
- Each window has a GL context
- Renderers can be bound to a window (identified by ID)
- Drawing stuff on the windows:
    - a closure, associated to the window, that has access to the application state (except the windows)
        - read the contents, draw stuff
    - can be multiple layers
    - A table of 2D "DrawLayers", associated to a window ID
    - WinHandler: has access to the DrawLayers
        - executes all draw layers
        - a draw layer can request an animation frame
    - can move DrawLayers between windows
    - DrawLayers are ordered, the order is defined explicitly by setting an integer priority
- Listening to input:
    - inputs are passed to registered handlers
- Register keyboard shortcuts

- Windows display things from the data model
    - update: data model revision
    - event: window event
- There is a global table of windows
    - on data model update, call update() on all windows in this table
- The run loop calls the event handlers for all windows
    - must borrow dynamically the application state here
    - in event handlers, must be able to open other windows
- RunLoop
    - AppState
        - Open documents
        - Open windows
    - Registered views on the application state

```
trait WinHandler {
    event(&mut self, ctx, event, &mut appState)
    paint(&mut self, ctx, appState)
} 
trait View {
    update(&mut self, Revision<appState>) -> Action
}
```

## High-level architecture
The application is divided in components that communicate via a shared event bus.
Examples of components:
- Document model 
    - Open documents, etc.
- User interface
- Renderer
- Network
- Etc.

The document objects also "make space" for the data required by application components.
So that when a document object is deleted, the data for each component is also deleted.
With a "distributed" approach (multiple ID -> Data maps), need to listen to events to 
delete the associated data (i.e. synchronization).

Rule of thumb: if we know in advance that there is going to be only one instance of the component data, 

Behavior:
- The top-level program gives control to the UI component, which then returns an action to perform (unidirectional event flow).
- The action is translated into a command that is sent to the document.
- The document emits events to signal changes
    - emits where? 
    - the document model component has an observable that contains a list of handlers
        - problem: can't store handlers having exclusive access to components
    - the components operate as cooperative tasks
        - not exactly a task, as sending an event blocks the caller
        
        
## Kyute windows
- Q: can they be created within the widget hierarchy, or only as top-level?
    - If they can be created within the widget hierarchy, then it must have access to the ambient state
        - and all window events must be rerouted to the root of the hierarchy for propagation
    - If windows are "special", then they are self-contained, in that they can hold a strong ref to a part of state
- Q: modal windows (and widgets)
    - modal dialogs require platform support, which druid-shell doesn't have right now
    - combo boxes?
    
## What do we need from the window system?

- (ignore mac and linux for now)
- can create an OpenGL context
- can render 2D graphics with D2D/DirectWrite/whatever native API is there on linux
- create native menus (main menu bar + context menu)
- create borderless windows for combo boxes and stuff
    - and draw to it
- receive events from graphics tablets
    - WM_POINTER
- native dialogs
    - file save/open
- native font rendering


## The great `Data` change:
- Make it `?Sized`.
- Remove the `Clone` bound.
-> that was surprisingly painless

## And now, the renderer (again)
- Q: Should we have objects that wrap OpenGL resources, and delete the resource on drop?
    - autograph-ng is still a thing
- There are good ideas in autograph-ng
    - the arenas are NOT one of them: unusable in 'static contexts
        - application structure is constrained to a set of nested loops
        - this is incompatible with druid
        - can't delete one object at a time
        - all this to avoid putting a backreference for deletion
- There are types to expose to the nodes, and types to keep private in the renderer
- Useful types in the backend:
    - management of textures/buffers (or Images: Renderbuffer OR Texture)
        - a base handle type, which only has a deleter, when things need to be kept very lightweight
        - plus a simple wrapper to create one
    - management of framebuffers
    - don't use backpointers: use a global variable
    - some convenient abstraction for shader state

- Rendering interface: TODO

## Consider WebGPU (wgpu-rs)
- Issue: extensions 
    - imported memory (EXT_memory_object)
    - interop with other stuff
- Made for the web, portability first
    - might want more flexibility
    
## Should we even use OpenGL?
- context creation / multi-window is hard
- interop with D2D (druid) might be hard
- modify druid_shell to provide a D3D11 context instead


## API options
- OpenGL
    - has GLSL, there exists a GLSL parser for Rust
- D3D11
    - good interop with D2D
- ~~Vulkan~~: too complex
- 

## presentation
- application emits draw commands
- commands are flushed to the GPU queue
- SwapChain::Present is called
    - what happens here? does present wait for the queued ops on the GPU to finish?
    - probably not: 
    
    
## Existing applications
- Maya: DirectX, OpenGL
- Blender: OpenGL
- Nuke: OpenGL
- Natron: OpenGL
- Cinema4D: OpenGL, Metal
- Unity: DX, OpenGL, etc.
- Houdini: OpenGL (multi-context, also for UI?)
- Substance: OpenGL (?)

## Reconsider using winit?
- druid-shell
    - is smaller than winit
    - has native menus
    - has native file dialogs
    
- winit
    - does not provide any rendering context: we do that ourselves
    - was there for a longer time
    - seems less complex than druid-shell w.r.t. drawing?
    
- ... and winit is in a fucked-up state.

## Don't use piet/piet-d2d
- latest version doesn't seem to support DxgiRenderTargets
- use directwrite/direct2d directly
    - don't really care about linux support for now
    - wait for piet to grow up
- don't invest too much time in directwrite/direct2d
    - eventually, move away from it, wrap dwrite/d2d in a tailored API for kyute+artifice from winapi
    - less code to maintain, less layers, less things to work around
    
## TODO
- PlatformWindow: impl WindowEventTarget, wrapper for paint, draw
    - PlatformWindow(DocumentWindowHandler)
- UI: current focused modal window
    - PopupWindow(PlatformWindow(PopupWindowHandler))
    
    
    
## The scope is too big
- Custom UI with layout, input handling 
- Custom windows, menus
- Custom renderer
- Data model
- Lens-based change tracking system
=> too much

What we want:
- load a 3D scene in memory from a file
    - along with a recipe for rendering objects on the screen
    - recipe is more complex than simply a material: both material attributes and control targets that talk to specific post-proc passes / renderers
    - stroke placement and rendering
- User interface to edit the recipe

Shortest path:
- use IMGUI for the GUI

The custom UI is too much. Too many decisions to make regarding the design of the API, the layout system, painting, state management, etc.
Should go with a simpler approach? immediate mode?

immediate mode:
- on state change / event received
    - updates the previous layout
    - invisible items are skipped (although this must be conservative)
    - functions like button(), etc. also take a callback that produce an Action
    - in the end, simply produces a bunch of boxes
    - identity using Identifiable trait
- problem: visibility determination without layout?
- layout the boxes and text within the boxes
- draw the boxes and text


## Follow-up
- don't follow up with the model+lens thing
    - use ad-hoc change 
- don't follow up with the view stuff


## The document model, in Rust
- Compromise between
    - a very simple document model, with very few primitives, and extensible with schemas: GENERICITY
        - (+) load/save is dead simple - no code on the extension side
        - (+) UI can be generated automatically - no code on the extension side (if desired)
        - (+) more generally, low-level tools can manipulate the structure without knowing the high-level schemas
        - (-) Overhead: every object is dynamic
    - a document model with lots of different objects
        - (+) less overhead
        - (-) load/save of objects must be reimplemented for every type (or auto-derived)

- Rule of thumb: user first
    - favor first flexibility and extensibility for the user, then performance

- Basic concepts:
    - Node, which have Attributes
    - Attributes have a type and a value of that type
    - Nodes have metadata
    - Attributes have metadata
    - Attributes can be a pointer to another attribute 
    - Nodes have paths

- The interface of nodes (IOPs) is defined by the type of the node, makes no sense to edit them
    
- On top of that, components can be added to nodes (interfaces)

    
    
- E.g. a shader node
    - Node trait:
        - Input attribs
            - +Metadata
        - Output attribs
            - +Metadata
    - Editor trait:
        - node X position (attribute)
        - node Y position (attribute)
        - color           (attribute)
    - ShaderNode trait:
        - source code (can be an attribute, can be evaluated)

- Problem: a lot of things can be attributes, and can be bound to an expression, etc.
    - Given a Node (primitive type), create schema objects that access an aspect of the node
    - schema ~ traits
    
- Attributes that can be both a connection or a value?
    
```
let node = doc.node("/shade/img0");

let shader = Shader::from(&node);
// OR (to create it)
let shader = Shader::create(&node);

// then
shader.glsl_source();
shader.uniforms();
```
        
- hierarchy of objects with attributes
- attributes can be values, expressions, or references
- an object within a hierarchy is identified with a path
- the structure of a node is controlled via one or more schema classes
    - this is done so that, if the schema is not there, the document is still viewable, and no information is lost
- node handles are both mut and non-mut: RefCell-like 
    - locks the whole system on borrow
- changing the schema in response to a parameter change?
    - [Model borrowed] schema listens to parameter changes
    - [Model borrowed] schema schedules a "schema changed" action -> goes through the action dispatcher
- don't discard unknown properties in the input file
    - load in memory into generic, hierarchical key-value pairs
    - Option 1: loader: deserialize into the in-memory model
        - how to know which properties have been used, and which ones are unknown?
            - through a custom API
        - 
    - Option 2: loader: deserialize into a generic model, apply schemas on top
        - zero-copy possible
        - everything is loaded
        
        
## Rendering of purely cosmetic visuals
A lot of widgets are composed of an outer frame, and a "content area" inside, 
with some padding between the two.
Is it necessary to have three visual nodes (one for the frame, one "layout box" for the padding, and another for the content)?
-> currently, yes, because of the way layout is done right now

-> make it simpler 
    - given constraints:
        - apply padding on constraints
        - size and position the inner box
        - add padding

-> nested layout boxes
    LayoutBox
        
    let b = LayoutBox::new(constraints);
    b.inner_constraints();
    let outer_size = b.set_inner_size(inner_size);
    
    let box = PaddingBox::new(constraints, |inner_constraints| {
        ... return inner visual, which has a size ...
    });
    box.inner_bounds();
    box.outer_size()
    
    visuals:
    - Frame
    - Text
    

        

Other option:
-> Frame{ inner: Padding::new(5.0, }
 
Other option: 
-> move box model (padding, border, margins) inside the node itself?
-> many widgets use some kind of box, except some layout widgets:
    - Align
    - Baseline
    - Padding
    
Align creates a "LayoutBox" that fills the parent, and places its child within it depending
on some alignment value. 
Baseline layouts the child, then creates a box that contains the child + some slack to place the child at a fixed baseline
height.

=> Instead of "wrapper" widgets, put alignment options directly into the node (like CSS).
=> this delegates the alignment computation (placement) to the container widget

Basically, move more things into the node, and remove redundant ones
Node:
- Alignment
- Margin
- Padding
- Border

Instead of node layout, have a node.layout: LayoutBox, specifying the size of the border and the size of the padding.
pass `LayoutBox::inner_constraints()` to the child widgets.

The API around "layout" is still not very good:
there are a bunch of members in "NodeData" that are accessible, and it's unclear which ones should be changed.
Notably, changing node.layout.offset has no effect because the offset is overwritten by the parent.
Ideally, layout() should return a visual and a "Box" (content size + padding?)

List of confusing public fields of NodeData accessible during layout:
- window_pos (overwritten later)
- layout.offset (but not layout.size)
- in some way, key

The problem with themes is that you need the values at different steps.
Let's take the example of a border size shared between elements:
- during layout, you need the border size to adjust the constraints passed to the child widgets
- during painting, you need the border size to know the size of the border to paint
=> The solution is to make the border a separate widget (like flutter).


## Changing the API of "Widget::layout"
Cannot really be changed, because, as previously found, we need to pass a cursor so that the child widget can find
its matching node in the tree.
Why is it the responsibility of the child widget? => because the child widget always knows the concrete type of the 
visual that it produces, and we use that type for reconciliation.

So we really need to change the API of cursor.reconcile()
The current API is `reconcile(F), where F: Option<NodeData<Visual>> -> NodeData<Visual>`.
Proposed API, more precise:
`reconcile(F) where F: Option<Visual> -> (BoxLayout, Visual)`
Under the hood, it updates the size in the node. `NodeData` becomes an implementation detail.

## Ensuring that "Widget::layout" inserts only one node
It's unclear whether this restriction is necessary or not:
- if it is: then the cursor passed to layout should only allow the insertion of one node
- it it's not: then the function should not return a (single) NodeId
 
```

// the event loop should have a ref to the windows, so that it knows where to deliver
// the event based on the window ID.
//
// a child window itself is conceptually "owned" by a parent visual.
//
// actions can be emitted by a window, but not during a traversal of the whole tree
// (only the subtree associated to the window), so action mappers can't operate during the traversal.
// Solution: action mapper has an Rc<ActionSink>
// - one root action sink, which is ActionSink<RootActionType> + one sink per mapper which forwards
//   the transformed action to the parent sink
//      - problem: potentially a lot of mappers, one Rc for each
//
// other option:
// - accumulate all generated actions in a vec alongside the window, then
//   signal the parent window that a child window has generated actions
//   then, traverse widget tree of parent window, and collect (and map) generated actions
//
// other option:
// - always propagate events starting from the root
//    for windows, it means that the event may need to traverse the whole tree before finding the child window
//
// other option:
// - nodes in the visual tree have paths, so that an event that targets a window can be delivered
//   efficiently to the node
//      - similar approach in xxgui
//      - problem: the structure of the visual tree is opaque, so need additional code in Nodes?
//
// There is actually a bigger problem, which is delivering events directly to a target node in the
// hierarchy, without having to do a traversal.
//  - can be useful for keyboard focus, delivering events to a particular window, etc.
//  -
//
// -> This means that visual nodes should be "addressable" (identifiable + an efficient way of reaching them)
// -> which is very hard right now, because
//      - A: the tree is opaque (traversal is the responsibility of each node)
//      - B: nodes don't have a common related type (there's Visual<A>, but 'A' varies between nodes).
//      - C: the layout boxes are computed on-the-fly during traversal
//
// B: The "Action" type parameter should not be in the nodes?
// A: The node hierarchy should be visible: have an explicit tree data structure?
// C: the calculated layout should be stored within the visual node
//
//
// Review of existing approaches:
// - druid: opaque tree, forced traversal to find the target
// - iced: transparent layout tree, no widget identity
// - conrod: graph, nodes accessible by ID
// - ImGui: forced traversal
// - Qt: probably pointers to widgets
// - Servo DOM: tree, garbage collected
// - Stretch (layout lib): nodes are IDs into a Vec-backed tree
// - OrbTk: IDs in an ECS
//
```


## The artifice pipeline language

It's a language that allows the user to specify a sequence computations to run on the GPU, and resources to allocate.

The basic building block of GPU computation is a _pass_. A _pass_ reads from and writes to _resources_.
_Resources_ are blocks of memory on the GPU. There are two main types of _resources_:
_images_, and generic _buffers_. 

Multiple passes are then combined in a _schedule_, that describes which passes to execute, and in which order.

Declaring an image is done like so:
```
image Normals;
image Depth;
```

The precise type of the image is inferred through usage.

_Modules_ group resources and passes into a common namespace.

```
module g_buffers {
    image normals;
    image depth;
    
    pass gen_g_buffers {
        ...
    }
}
```

_Templates_ take parameters and produce modules and passes when "instantiated".

```
macro g_buffers(name) {
    module $name {
        image normals;
        image depth;
    }  
}
```

Technically, a simple "configuration" language would be enough; it should support macros and includes, though.
_Modules_ are optional.

Something more akin to a "scripting" language would be OK as well. The question is how much flexibility and control we
want to put in the file.

E.g. iteratively downsampling a texture:
- in script: need a loop and expressions
- in program 

## GUI themes

Theme is an interface to draw and layout reusable visual elements called frames.
It defines the metrics (size, padding, margins) of these elements, and methods to draw them.
Ideally, it should also be able to "override" the rendering of custom widgets.

Possible signatures:
```
fn draw_frame(&self, ctx: &mut PaintCtx, bounds: Bounds, class: Class, params: &dyn Any);   // very similar to QStyle
fn draw_frame(&self, ctx: &mut PaintCtx, bounds: Bounds, class: &dyn Any); // class determined by the type of Any
```
where params depends on the class (can be anything, passed by the visual during rendering).

Bikeshedding for `frames`:
- Primitive / primitive element (Qt)



## Kyute consolidation phase
A lot of things were added recently to kyute and kyute-shell:
- the environment, with a complex system for locally overriding keys
- a rudimentary styling system (StyleCollection) that can be loaded from files
- matching additions to `kyute_shell::drawing` to create gradients.

The purpose of this section is to collect various pain points and inconsistencies with the current API and alleviate them.
- metrics should be distributed along the style collections
    - remove them from the environment
- ~~the Point,Offset, etc. types should be moved into kyute-drawing and assigned a "DIP" unit.~~
- ~~a layout debugger (show element bounds)~~
- make the slider work also vertically
- introduce a scoped version of DrawContext::save/restore
- normalize passing by value and by-ref for geometric types (Bounds?)
- error handling in the drawing library
- maybe split Platform context into sub-contexts? (for Drawing, Text, etc. => PlatformDrawing, PlatformImaging, PlatformText)
- DrawContext should reference the PlatformDrawing
- move styling module in a separate crate?
- specify the event delivery, propagation and focus logic
- ~~replace `Bounds` with `Rect` for consistency~~
- provide the `ToDips` that converts a type to a DIP size given a target.

## Remaining widgets for "self-hosting" a style editor
- Combo box 
    - drop down
    - Popup windows
- Checkboxes
- Radio buttons
- Menus
- File picker
- Table view

## Testing
???

## Pros/cons of changing the drawing API to use strongly-typed units

Pros:
- self-documenting

Cons:
- lots of noise when calling (must wrap all lengths with DipLength)

Variant: drawing functions take IntoDips, where f64: IntoDips, f32: IntoDips, ... Length<Dip>: IntoDips

For convenience, leave f64 for now.

## Sharing a handle to the node tree to component tasks
Need to wrap the `NodeTree` into `Rc<RefCell<>>`, and _pass it to LayoutCtx as such_, 
which is **supremely annoying** (borrow_mut and weird derefs everywhere).
Ideally, we want a  async task that is somehow resumable with a mut ref to the NodeTree as the context, 
but that's not going to happen anytime soon, is it?

## Multi-threaded component update
- Each individual node needs to be wrapped in `Arc` for concurrent access.

## Don't store component state in the visual tree?
- Instead, put them in a secondary map: components don't need the tree to update themselves, 
  only their state
- Problem: reconciliation/update needs a special API for Widget::layout
    - update component state / get previous state
- Move the state in the task itself?
    - problem: prop update
  


### Descriptor set layouts, pipelines, etc.
- global cache

### Pipeline query builders
```rust
struct MyPass {
    tex: ResourceId,
    pipeline_variant_a: PipelineQuery,
    pipeline_variant_b: PipelineQuery,
    pipeline_variant_c: PipelineQuery,
}
```
- the pipeline create infos need a bunch of slices, and thus borrows or vecs or arrays:
    - vertex binding descriptions
    - vertex attribute descriptions
    - viewports
    - descriptor set layouts
    - push constant ranges
    - render pass subpasses
- Some of those we might want to infer from the SPIR-V
- Q: Why do we need queries anyway?
    - just pass everything in one go and get the result
    - cache individual shader modules, renderpasses, descriptor sets

- Options:
    - borrows: `PipelineQuery<'a>`
        - lowest overhead
        - lifetime pollution
        - exposed to self-referential borrows when storing in a struct
    - nothing
        - it's probably one of those cases where it's easy to over-engineer things
    
## Shader bindings
- For textures: VK_EXT_descriptor_indexing
    - Big table of descriptors, one for each texture
    - Every shader-accessible texture is put into this table and assigned an index
    - The descriptor set is passed to all shaders as set #0
    - Pass texture indices as a shader parameter (push constant)
    
- For uniforms: 
    - Pass the device address to the buffer in push constants
        - Can have multiple buffers
    - No need to bother with allocating descriptor sets
    - Issue: might be less performant than uniform buffers on some architectures 
        - e.g. on AMD, there seems to be a dedicated constant cache 
            - see https://developer.amd.com/wp-content/resources/RDNA_Shader_ISA.pdf
                - Chapter 9. Flat Memory Instructions
        - other archs: how the fuck can we know
        - Generates different instructions:
            - uniform buffer: s_buffer_load_dword, scalar load
            - buffer_reference: global_load_dword, vector load
    
- Tentative design:
    - one big array of texture descriptors, always bound to set #0
    - when a texture is first used in a pass, add it to the big array
    - push constants contain pointers to storage buffers
        - (8b) pointer to per-frame buffer
        - (8b) pointer to per-material buffer
        - (8b) pointer to per-object buffer
        - (64b) matrix
        - (40b) free for other per-drawcall stuff
    - just allocate a buffer per-object, or suballocate if necessary
        - methods to 
    - put all of this in a convenience layer on top of the context
        - wraps add_resource_usage
    
- More traditional design:
    - same big array of texture descriptors (that doesn't change)
    - for uniforms:
        - set #1: per-frame uniforms
        - set #2: per-material/object uniform, dynamic uniform buffer (per convention)
        - push constants: per-drawcall
    - allocation of descriptors:
        - layouts inferred from SPIR-V
        - combine layout with an allocator:
            - track last serial for each set
                - the set should only be used in one queue (otherwise need to track multiple queues)
            - re-use if serial is completed
            - multiple pools, expanded as needed
            - not super hard to implement
        - layout+allocator combo is cached, by hashing the descriptor set layout
        - one pool per layout? is that OK?
            - approach taken by unreal engine and others, seems reasonable
    
## Shaders and pipelines: expected interface
It's not super useful to fully create a pipeline layout from reflection,
because the application that uses the shader usually knows what to expect from the shader.
Reflection is useful when a shader has parameters that should be exposed as an UI, but not much more.

A more useful thing would be to ensure that a shader/pipeline **conforms** to a particular interface, 
defined **in code**. Reflection is still needed but only for verification, and dynamic stuff like uniform UIs.
Reflection stuff should probably be moved in `graal::spirv::shader_interface` or at least in a separate module.
-> the autograph-ng legacy lives on

## Code organization
- device.rs: device stuff
- swapchain.rs: swapchain related stuff
- context: create / track resources, batches, passes
    - context/mod.rs: Context, ResourceId, SwapchainId, SubmissionNumber, QueueSerialNumbers
    - context/batch.rs: Batch
    - context/pass.rs: Pass 
    - context/resource.rs: resource creation
        - buffer/images
    - batch
    - pass builder
    - serials  
- descriptors: descriptor management
-> wait until more descriptor stuff comes around
    -> easy enough to refactor at that time 
  
## Idea: fully static pipelines (macro-batches?)
- Specify the whole render pipeline statically, indicating what can change and what is fixed
- A whole DSL?
- Can optimize for render passes automatically, layout transitions, scheduling
    - no need for per-frame computations
- e.g. "load image from a file"
    - produces one resource, list of possible output formats
- automatically handle the combinatorial explosion of pipelines/render targets
- conditionals
- e.g. rendering:
    - inputs:
        - format 
        - previous frame in the given format
        - vertex buffers
    - outputs: new frame
    
Assume you have a shader with some SPIR-V, but you want to run it on targets with multiple formats.
You need a different renderpass, and a different pipeline.
You don't want to deal with that, you just want someone else to create the renderpass and pipeline 
that corresponds to the target format.

So you declare your pass like so:
```
- one input image, with a format that is one of those: [list of supported formats]
    - it should be in the following layout
    - i'll need to know its size
    - i'll need to know its precise format
- one output image, with a format that is the same or similar to the format of the input image
    - it will be in the following layout
    - it will have the following size, calculated from the input size

- I will need:
    - one render pass, for one target, with the format of the output image
    - a pipeline
        - with these modules: (spirv file)
        - with the render pass specified above
```

Another example: let's say you want to load an image file. 
You need an output image on the GPU to store it. However, you don't know in advance what will be the format 
of the image (usually you should know the data that you consume, but let's assume it's a user-facing thing).
You declare your pass like so:
```
- one output image
    - I don't know its format, but I know that it can be one of those <list of possible output formats>.
    - I also don't know its size.
    - I'll need it to be in the `TRANSFER_DST` layout, please 
    - It will be in the TRANSFER_DST layout afterwards
```

In the pass, you'll need to fill in the remaining details about the size of the image, etc. 
before you can get an actual vkImage.
However, since you specified some info about the image, other passes can make decisions from that info.

Matching an input and an output is like solving a system of equations.
You have an input image, with a set of possible formats, size, etc.
and layouts.
You need to verify that 1. the size is the same, 2. the formats are compatible, and also to generate layout transitions

Pipelines: made from GLSL shaders.
Probably should support "shader variants" made from macros.

## Case study: a downsampling compute shader (depth-aware)
The pass has an input texture, which is downsampled to the given resolution (possibly down to 1x1).
The input texture must be a floating-point format (f32, unorm, snorm), and must be compatible with sampling.
The pass will create one output texture with a compatible format.


### Compilation variants
So, first, the output image in the shader must be declared like this:
```glsl
layout(set=x, binding=y, rgba32f) uniform image2D outputImage;
```
Note the format (`rgba32f`): this is hardcoded in the source. So it needs to be a macro if we want it to support multiple formats.
```glsl
layout(set=x, binding=y, $output_format) uniform image2D outputImage;
```
Hence, the shader source now expects a variable to be set, `$output_format`. The provided source does not represent a shader module, but rather a template for shader modules (a `.glsl.in`).
There are variants for each `$output_format`.

### Specialization constants
The compute shader has a workgroup size definition:
```
layout(local_size_x = 16, local_size_y = 16) in;
```
The size of the workgroup should match the SIMD size of the underlying hardware. 
It could be set with a variable, but for this we have specialization constants:
```
layout(local_size_x_id = 0, local_size_y_id = 1, local_size_z_id = 2) in;
```
Specialization constants 0,1,2 are used for the workgroup size. Those are provided during the creation of the pipeline.

### Uniform buffers
```
layout(set=x, binding=y, std140) uniform U0 {
 ivec2 origin; // upper-left corner of footprint
 ivec2 size; 
 float opacity; 
};
```

## Case study: rendering a scene
The shell (rust code) provides the data to render, but the format is not known in advance.
For instance, it can be 
    - "position only", or "position + normals"
    - "triangles", or "lines"
All possible permutations are declared beforehand. 
Then, rust code can get which permutation fits the input data, **at runtime**.
=> but then, this may have an influence on the rest of the passes!
    - the goal is that it shouldn't
    - who/what is in charge of running the draw command?
        - the shell (application)
    - drawing a mesh is only a matter of uploading the data into a GPU visible buffer (not our responsibility), 
      and to run the correct pipeline.
        - query the corresponding pipeline variant at runtime 


## Base concepts:
- Pass
- Data context
    - Can extract variables from this data context

## Problem: variable number of passes
- For example: 
    - painter_v2 for shading design
    - Flair
- Q: How do we do this?
- A: don't
    - no interactivity that involves adding more passes
    - OK for video games and such, but what about prototyping?
- A: some complicated mechanism with loops
- A: runtime code generation
    - basically call rustc?
- Q: is everything worth it?
    - we get the ability to: 
        - create optimized render passes statically
        - handle shader permutations statically (precompile every permutation)
        - codegen minimal and efficient synchronization code 
    - at the cost of:
        - not being able to add new passes or shaders at runtime
        - not being able to run a pass in a loop
- Q: is the cost of computing synchronization per frame measurable?
    - a dubious claim

## Effects/techniques/whatever

An effect is a combination of a pipeline and one or more passes.
In order to run an effect, you need:
- (context) descriptor set layout cache
- (owned) descriptor set layout
- (owned) descriptor set layout allocator
- UBO ID
- mapped pointer to UBO
- buffer handle to UBO
- shader interface struct
- descriptor set
- (owned) update template
- get handle of output image
- create output image view
- create framebuffer

Constant:
- descriptor set layout
- pipeline layout
- pipeline
- some descriptor sets?
Those are stored in the technique object, they are long-lived.

Resource-associated objects:
- output image view
- framebuffers
Those are not put in the technique.

Parameters:
- format of the output image
- format of the input vertices

```rust

fn main() {
    let effect_template = EffectTemplate::from_file("...");
    
    let mut current_size = unimplemented!();
    while !should_stop {
        
        let window_size = unimplemented!();
        if window_size !=  current_size {
            current_size = window_size;
            
            // communicate the size change to the effect
            // internally, the pipeline object depends on those
            // two values, so setting those two values will 
            // invalidate the created pipeline.
            // This means that the pipeline will be re-created. However, 
            // re-creation is not immediate, but deferred to when the pipeline object is actually needed.
            effect.set_width(window_size.width);
            effect.set_height(window_size.height);
        }
        
        
        let mut batch = context.start_batch();
        
        // allocate buffers
        let buf = unimplemented!();
        
        // setup effect, could be typed
        effect.set_vertex_buffer(buf);
        effect.set_param(12.0);
        effect.set_output_image(swapchain_image);   // this will hold a strong ref to the image
        
        // issue: setting references to resources
        // -> the effect ends up owning a strong ref to the reference, even if it's not going to be used
        // for more than a batch
        // -> pass resources in `Effect::run()` instead?
        // Problem: for some resources, it actually makes sense for the effect to hold a strong ref to it
        // -> long-lived uniform buffers, textures, etc.
        // -> no single owner: can write to an uniform buffer while the effect references it
        
        // this runs the pass: internally, it will create and/or update
        // any invalidated object (pipelines, framebuffers, etc.) 
        
        let r = EffectBatchResources {
            uniform_buffer: unimplemented!(),
            vertex_buffer: unimplemented!()            
        };
        effect.run(batch);
        
        
        
        batch.finish();        
    }
    
}

struct Effect {
    // pipeline
    pipeline: Option<vk::Pipeline>
}

```


```

// this could be a memoized function
// it'd be cool if all render effect files could share the same instance of the render pass with a particular format
// e.g.
// effectA.fx: color_only_render_pass(RGBA8) 
// effectB.fx: color_only_render_pass(RGBA8) 
// -> the two render passes refer to the same variable, and are created once
// -> that's hard, because the resources and objects live in an EffectTemplate object, and
// each file produces a different EffectTemplate

// can be called from rust:
// `effect_template.get_color_only_render_pass(device, format) -> RenderPass`
RenderPass color_only_render_pass(format) {
    Attachment color {
        flags = MAY_ALIAS;
        format = format;
        load_op: DONT_CARE;
        store_op: STORE;
        stencil_load_op: DONT_CARE;
        stencil_store_op: DONT_CARE;
        initial_layout: COLOR_ATTACHMENT_OPTIMAL;
        final_layout: COLOR_ATTACHMENT_OPTIMAL;
    }
    
    Subpass {
        color_attachments = [AttachmentReference {
            attachment = color;
            layout = COLOR_ATTACHMENT_OPTIMAL;
        }]
    }
}

DescriptorSetLayout background_shader_set_interface 
{
    type = BackgroundShaderInterface;
}

ShaderModule vertex {
    source_file = ...;
}

ShaderModule fragment {
    source_file = ...;
}

PipelineLayout pipeline_layout {
     set_layouts = [background_shader_set_interface];
}

VertexInputState vertex_input_state {
    
}



Pipeline background_pipeline {
    
}

pass background
{
    
}
```

Two points of view:
- effect writer:
    - doesn't care about where the resources come from, only that they are in a format that it supports
    - doesn't care about how long the resources live, only that they live at least until the effect has finished rendering
- "host" application:
    - wants to manage
    
## Do we really need a generic effect file format?
What we really want is a bridge between the host application and a piece of shader code or pipeline state that's configurable
at runtime.
We also want a convenience framework so that dealing with minor variants of pipelines is less painful. For instance:
- same shaders, but different render target formats (need a different render pass, and thus a different pipeline)
- same shaders, but different blend modes or rasterization options

Fact: a simple set of (cached) functions that take parameters for each piece of state would work.
E.g. :
```rust
// Caches the result somewhere
#[graal_fast_cache]
fn create_single_output_render_pass(
    cache: &mut graal::ObjectCache,
    output_format: vk::Format,
) -> vk::RenderPass {
    // ...
}


// transparently rewritten into:

// Option A: externally controlled caches
static create_single_output_render_pass_CACHE_ID : OnceCell<LocalIndexedCacheId<vk::RenderPass>> = OnceCell::new();

fn create_single_output_render_pass_2(
    cache: &graal::DeviceObjectCache,
    #[fast_lookup]
    #[allowed_values(vk::Format::R8G8B8A8_SRGB, ...)]
    output_format: vk::Format,
) -> vk::RenderPass 
{
    
    let index = match output_format {
        vk::Format::R8G8B8A8_SRGB => 0,
        // ...  
    };
    
    let local_cache_id = create_single_output_render_pass_CACHE_ID.get_or_init(|| {
        cache.create_local_cache() 
    });
    
    let local_cache = cache.get_local_cache(local_cache_id);
    
    if mem::size_of::<vk::Format>() < 8 {
        
    } else {
        
    }
    
    let mut __hash_key = Hash::new(); 
    __hash_key.append(source_location!());
    __hash_key.append(output_format);
    
    cache.get_or_insert_with(
        __hash_key, {
            // pasted code block
        })
}

// Option B: static caches
static create_single_output_render_pass_DEVICE_OBJECT_CACHE: OnceCell<DeviceObjectCache<RenderPassId, 4>> = OnceCell::new();

fn create_single_output_render_pass_3(
    context: &graal::Context,
    output_format: vk::Format,
) -> vk::RenderPass 
{
    let index = match output_format {
    vk::Format::R8G8B8A8_SRGB => 0,
        // ...  
    };
    
    let cache =  create_single_output_render_pass_DEVICE_OBJECT_CACHE.get_or_init(|| DeviceObjectCache::new());
    
    
    
}


```

Problem: where to put the local caches?
- owned by a global cache object: can ensure that the objects are deleted before the device that created them
- static: device objects effectively 'static: meh
    - objects must have a strong ref to the device that created them
    - Arc<Device>
    - DeviceObjectCache
    
Option A: externally controlled caches
- (+) explicit control over the lifetime of the objects
- (-) harder to implement 
- (-) less straightforward to access the local cache
    - need a first lookup by cache ID, then a downcast to the correct local cache type
    
Option B: static caches
- (+) simple
- (-) no explicit control over the lifetime of cached objects
- (-) must explicitly reference the device

Caching: generic or only for device objects?
- Problem: management of lifetimes

Problem: lifetime of cached objects
- example: cached function that returns a vk::Pipeline: for how long does the vk::Pipeline live?
    - indefinitely? memory usage can grow
    - LRU cache? can accidentally put the vk::Pipeline in a struct and use it after it was deleted, or when 
      it's in use by the device
-  

Lifetime of vulkan objects:

| object | owner  | ref in commands | depends on |
|--------|--------|-----------------|------------|
| sampler     | device  | yes | nothing
| image view  | image   | yes | image
| pipeline    | device  | yes | nothing?
| render pass | device  | yes | nothing
| framebuffer | device  | yes | render pass, images
| shader mod  | device  | no  | nothing


Simplification:
1. there's a single logical device in the whole application (excluding those in dynamically-loaded plugins)
2. there's a single context in the whole application
3. most methods in context are thread-safe
4. there's a global batch index counter in the context
5. the context contains slotmaps for every type of object
6. the context tracks uses of every object so that they are not deleted while still in use
    - problem: tracking each object individually is not efficient
    
The context should own the caches?
    

Each object should have a batch tracking number.


## Lifetime of render pass and descriptor set layout objects?
- Would be great if we could just allocate them and just forget about them forever
    - Memory usage? Probably not much
- They can always be associated to types
    - Note that pipelines cannot, or at least not easily:
        - You'd need a shit-ton of metadata on the type to describe a complete pipeline
        - Also makes it harder to have variants
        -> not everything in PipelineCreateInfo is for describing the interface with the host
- We still need to delete them on context drop.


## Lifetime problem with transient resources and passes
Transient resources bound to the lifetime of `Batch`, so they borrow it.
Passes need exclusive access to the batch (by construction), but it is already borrowed.

Solutions:
* don't require an exclusive borrow to build a pass
    * need to build a list of accesses and then "commit" to the dependency graph
        * not in-place, may require dynamic allocation
* two-stage design:
```rust 
fn main() {

// add_pass(&self, ...)
// RefCell borrow inside
batch.add_pass(|ctx| {
    ubo.access(ctx, ...);
    
    ctx.set_commands(|ctx| {
        // ideally, statically ensure that only the registered resources
        // are accessible here. branding?
        // -> would need a wrapper for all operations
        // for now, unsafe.
        
    });
});

}
```

## All vulkan mistakes:
- using a descriptor in a shader stage not specified in the layout => ERROR_DEVICE_LOST
- not providing clearValues on beginRenderPass => crash in validation layers
- forgetting to set the colorWriteMask in VkPipelineColorBlendAttachmentState

## Synchronization of uniform buffers
- right now one buffer per uniform block, might not be very efficient
    - upload pool


## Next steps
- DONE Figure out why the mesh is Y-flipped (is it a problem with the mesh, the camera, the viewport setup, something else?)
    - vulkan flipped the Y coordinate of the clip space: flip Y in vertex shader
- Setup TAA shader pipeline
- Hot-reloadable shaders
- Scene abstraction
- Generalize G-buffer generation from meshes
- Textures
DONE Camera control
- Figure out what we want to try with the style:

    1. Anchor point generation
        - load from file?
        - use a procedural solid noise technique
    2. Stroke mesh generation
        - anchor point to meshes
    3. Coarse stroke rasterization
        - Follows what?
            - need a good 2D screen space flow (tangent space?), bilateral-filtered
    4. Stroke DF resolution
    
## Problem with automatic resource transitions
Scenes have many vertex buffers, and currently they should ALL be registered when creating the pass.
There can be 100s of buffers.

Facts:
- vertex buffers are immutable and don't really need synchronization
    - we can just ignore them
    - actually no: we must still synchronize on the first data upload!
- images are a bit more problematic, because in theory they could be in different layouts
- put an explicit barrier on pass exit, which will ensure that the data is visible in all subsequent passes 
    

Problem 2: a pipeline barrier on pass exit is not enough because the using pass might run on another queue
- typical example: load on transfer queue, use on main 
    - a solution would be to do those operation on different _frames_ and sync them manually
        - problem: we introduce dummy frames which interfere with pacing    

Solution:
- introduce the concept of "immutable resources", which are resources that are written once and never touched again
    - those don't need synchronization
    - images of this type have a fixed layout
    - we don't need to introduce wrapper types for now, documentation will suffice
- provide a way to wait for a batch right after submission
    - batch future?

Another solution:
- when uploading immutable resources, just put a global memory barrier afterwards
    - can be done manually, totally transparent
    
## Figure out RAII for Scene
- Scenes own mesh buffers, but do not free them once it goes out of scope, simply because it doesn't have access to 
  the context at that time.

- Solutions:
    - Arc<Context> passed to Scene
    - global context singleton
        - why not? here are the possible reasons:
            - don't want the context to live for the whole program?
                - can still drop it manually (which automatically invalidates all resource IDs)
                - Rc<Context> also extends the lifetime of the context artificially
            - don't like singletons
                - not a reason
            - multiple contexts per program: which means, multiple vulkan instances (?) or multiple devices (?)
                - e.g. one device for integrated graphics, another for the discrete GPU
                    - technically possible, the worst kind of possible
                - for SLI: device groups
            - multiple shared objects (DLLs) that create their own context
                - basically, can't share resource IDs between DLLs
                    - it will go through a C interface anyway
- we're already making the assumption that there's going to be only one device anyway
    - interface types with static get_or_init()
    - lookup by type-id instead
    

- Problem with Arc<Context>
    - must really be Arc<RwLock<Context>> 
    - this lock-guards everything, even access to the ash::Device function pointers, which don't really need to be guarded
    

## possible confusion: two different kinds of batches
- "submission" batches (vkQueueSubmit), on one queue
- graal batches
- one graal batch may produce multiple submissions on different queues

Proposal: unify the two concepts?
- when creating a batch, specify the queue
- advantage: simplified concepts
- advantage: simplified code (no need to detect cross-queue dependencies anymore)  
- problem: worse intra-frame memory aliasing?
- problem: turning things "async" is harder, because need to split in different batches
- problem: pacing always considers batch N-2, but we now produce more batches in a "frame"
- problem: "transient" resources now only live within a submission, which is not a meaningful granularity for the user

Alt. proposal: rename "batch" to "frame" to avoid confusion with vkQueueSubmit batches
- OK

            
## The avail/visible tracking is wrong

We don't track resource state per-queue: a memory dependency in a queue will update the tracking info so that
another queue thinks that there's no need for a barrier even though it might not see the barrier yet.

Consider:

Pass 1, graphics queue : writes to A
Pass 2, compute queue  : reads from A, CQ sync, reset avail mask / set vis
Pass 3, graphics queue : reads from A, vis OK, no barrier
-> but pass 3 may run before pass 2, and before the memory barrier

Submitted in another order:
Pass 1, graphics queue : writes to A
Pass 2, graphics queue : reads from A, mem barrier, reset avail, set vis
Pass 3, compute queue  : CQ barrier, sync always OK
-> different behavior

Solution(?): don't set the availability mask after a cross-queue dep
-> other problem: on each read of a resource written on another queue, we add a semaphore wait 
-> this is very inefficient


### We add an execution dependency to writers, even when one exists already on the path to the writer
We need to scan the path to the writer to see if there's already a dependency.
One solution: update the reachability matrix on the fly, but that's a bit inefficient.

### The availability/visibility tracking is wrong, because of concurrent queues.

When accessing a resource we want to know whether there's a **dependency path** to the writer that includes 
a memory dependency to the resource access in question. And if there's not, we want to add one (or modify an existing
barrier on the path).
    - We also want to avoid redundant barriers in different queues 
    - Use split barriers

-> Use split barriers and track visibility per-queue?

Tracking memory dependencies:
- Q: is there a memory dependency for this resource and this access type between this pass and the last writer
    - read access:
        - 1. check the visibility mask for the queue
        - 2. if memory visible, nothing to do
        - 3. otherwise, insert a memory dependency
        - 4. reset availability mask for this queue, update visibility
    - writing:
        - 1. insert memory dependency
        - 2. 
    
    - alternate answer:
        - find a dependency path between the source (writer) and destination (access) and build the access scopes
            - if no path is found, must add one
            - otherwise, check access scopes to ensure that the first one (availability) contains all the writes of the writer
              and the second one contains all requested access types (visibility)
                - if not, must add one: either modify an existing barrier or create a new one

```rust
struct DependencyPath {
    passes: Vec<SubmissionNumber>,
    
}
```

`Batch::find_dependency_path(&self, from: snn, to: snn) -> Option<Vec<snn>>`
`collect_memory_dependencies(&self, resource: id, path: &[snn]) -> `


## Memory dependencies across queues

Typical example:
(1)Q1: write A
(2)Q1: read A          (3)Q2: read B

Submission order:
1,2,3 -> 

Q1: write A
avail mask: COLOR_ATTACHMENT
vis mask: Q1:empty, Q2:empty

Q1: read A
avail mask: empty
vis mask: COLOR_ATTACHMENT

-> don't reset the availability mask ?
    - consider only the availability mask of the writer pass


Exec:

- WAR dependency:
    - if single reader in the same queue
        - realize with the pipeline barrier of the destination stage 
    - else
        - realize with a semaphore
- RAW dependency:
    - if writer in the same queue
        - realize with the pipeline barrier of the first reader
    - else
        - realize with a semaphore
- WAW dependency:
    - if writer in the same queue:
        - realize with the pipeline barrier 
    - else
        - realize with a semaphore
    

                                          WAW     WAR     RAW
Same queue / single reader in same queue  PB      PB      FRPB 
Different queues / multiple readers       S       S       S

- same queue, both passes are local
    - realize with a pipeline barrier
        - WAW/WAR: the barrier is in the destination pass
        - RAW: the barrier is in the first reader pass 
- same queue, source is in a previous frame
    - realize with a semaphore
- different queues
    - realize with a semaphore
    

## Upload frames

```rust
fn main() {
    // 1. create the frame used for uploads
    let upload_frame = context.start_frame(...);
    
    // 2. use functions on scene, etc. to load stuff using the upload frame
    scene.import_obj(&upload_frame, ...);
    let img = load_image(&upload_frame);
    
    // 3. Finish the upload frame
    upload_frame.finish();
    
    // 4. 
}
```

# Bring graal to a higher level
The current frame abstraction for graal still doesn't know enough about the rendering to be optimal:
- it reallocates new resources for transients on every frame, even though they could be reused
    - this was made so that the graph could change on every frame
  
  
# MLR: draw calls

We'd like draw calls to be something like this:
```rust
fn test() {
    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct SceneArguments {
        // uniform variables will be put in a single uniform buffer, at location 0
        u_view_matrix: Mat4,
        u_proj_matrix: Mat4,
        u_view_proj_matrix: Mat4,
        u_inverse_proj_matrix: Mat4,
    }

    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct MaterialArguments<'a> {
        u_color: Vec4,
        #[argument(sampled_image,binding=1)] t_color: TextureDescriptor<'a>
    }
    
    
    // must either borrow SceneArguments or copy, since we can't create
    let scene_args = ctx.create_argument_block(SceneArguments {
        u_view_matrix: (),
        u_proj_matrix: (),
        u_view_proj_matrix: (),
        u_inverse_proj_matrix: ()
    });
    
    let render_pass = ctx.start_render_pass(...);   // specify (borrow) target images
    
    for batch in material_batches.iter() {
        let material_args = render_pass.create_argument_block(
            MaterialArguments {
                u_color: (),
                t_color: TextureDescriptor::new(&batch.texture, Sampler::linear())
            });

        for mesh in batch.objects.iter() {
            // issue: validation that batch.texture is in the correct state here.
            render_pass.draw(&[&scene_args, &material_args])
        }
    }
}

// what's annoying is that since resource allocation is delayed, it's impossible
// to create the descriptor sets before the time we build the command buffers.
// This means that we **invariably** end up with an Arc referencing something
// in the pass callback: the data to create the descriptor set, and the descriptor set
// allocator.
//
// Is this a problem?
// -> referencing the descriptor set allocator => there should be one per "command builder" thread anyway
// -> referencing the data to create the descriptor set => it's just ResourceIds or vk::Buffers


// what to do with frame resources? descriptor sets, image views, etc.
// => graal could reclaim them automatically?
// => manage them manually anyway


// Proposal: rewrite resource aliasing
// => right now, memory assignment is done at the end of the frame
// => proposal: perform memory assignment "on the fly"
//      - find a compatible resource that is discarded
//
// Advantage: the memory is bound immediately, can use the resource in a descriptor
// Drawbacks: different behavior depending on the order of allocations
//      - alloc small, discard, alloc big => will allocate two memory blocks
//      - alloc big, discard, alloc small => will fit small inside big
//
// This kinda goes against the idea of "having a full view of the frame" for better
// optimizations.
//
// But: what about allocating the memory, and also *freeing* the memory block on the
// allocator side? (but not really "freeing", just saying that it can alias with other resources).


// Other proposal: remove resource aliasing in graal altogether.
// - maybe it's not the right place
// - it is, however, the only place where we can do it at such a low level
//  (directly aliasing the underlying memory blocks).
// - there's probably a shitton of bugs inside the algorithm anyway
//
// How to do it at a higher level?
// - when we already have a graph structure
```

Problem: this render loop must be located outside of graal's command callbacks.
Still, we don't want to create a graal pass for each draw call.
Ideally: we want to coalesce draw calls that don't have memory dependencies between each other.
Problem: how to determine if two draw calls have a memory dependency?

context: store currently accessed resources, and access kind
on draw: if read-after-write or write-after-read

Proposal: coalesce draw calls if all bound resources are the same
=> or rather: build the set of all resource groups, and resources: if they are the same 


## Custom memory allocator
Allocate blocks of 64 MB on device.
Sub-allocate blocks.

When allocating, pass last submitted pass serials. 
E.g. serials 5 6 0 0. Meaning that we just submitted passes 0:5 and 1:6. 
0:5 allocated block A(0:5)

We want to allocate a block of memory for use in a subsequent pass.
First, on which queue are we going to use the memory? If we want to use it on pass 

We are at pass 0:7. 

At this point, we don't have any information about the execution dependencies of the current pass, 
so we can't really alias the allocation without potentially introducing false dependencies :(
We do have, however, information about the execution dependencies of the previous pass on this queue (0:5).

We know all execution dependencies of the previously submitted pass. 
Which really doesn't matter, since the current pass (0:7) may not have any exec dep on the previously submitted pass...
We basically have to rely on heuristics to alias the memory block.

OR, just tell the users of the library that they should register all dependencies before allocating a block of memory 
for the pass.
=> it's an API nightmare: depending on the order of calls, you get different optimizations.



Unfortunately, it's really easier to alias memory once we have information about the whole frame...


What about a simple heuristic? Given a memory segment like this:

111122224400011122

We're at pass 5, want to allocate a block of size 10 -> find a segment so that `max(sn)` over the serial is the minimum.
Pick `1111222244`, with `max(sn) == 4`. Then sync on pass #4 before accessing the resource for the first time.
With a bit of luck, we might need to be syncing on #4 anyway because of another resource dependency.

............................

Note that currently transient allocation is probably broken: we only update the pass predecessors for XQ deps 
(probably because within one queue, execution deps are between two stages)

E.G

SN #4: Read T0  - FRAGMENT_SHADER
       Write C0 - FRAGMENT_SHADER
Free T0
Allocate T1
SN #5: Read C0  - FRAGMENT_SHADER 
       Write T1 - VERTEX_SHADER     // why not

Execution deps:
`#4:FRAGMENT_SHADER -> #5:FRAGMENT_SHADER` via C0

Aliasing:
T0: #4
in pass #5: the required execution dependency for aliasing T1 with T0 is : `#4:FRAGMENT_SHADER -> #5:VERTEX_SHADER`,
but the one that we inserted, and added to the reachability matrix, only accounts for the weaker `#4:FRAGMENT_SHADER -> #5:FRAGMENT_SHADER`!

=> the current transient allocation algorithm is incomplete. To fix it: the reachability matrix should include source and destination stages.
1000 passes : 1000 x 1000 / 2 * 8 => 4MB table

`#4->#5: src_stage, dst_stage`
when adding a barrier: 
pass.preds.push((this pass, src, dst))

problem: `#4->#5: VERTEX -> FRAGMENT` + `#5->#6: FRAGMENT->COMPUTE` doesn't mean `#4->#6: VERTEX -> COMPUTE`

```
M_i,j = (src_stage, dst_stage)

* 4:VS->5:FS + 5:CAO->6:FS

                         4: DI VI VS TCS TES GS FS EFT LFT CAO
                                 <<X
             5: DI VI VS TCS TES GS FS EFT LFT CAO
                     <<Y            X>>                                          
 6: DI VI VS TCS TES GS FS EFT LFT CAO
                        Y>>

Y logically after X, so <<X->Y>> (4:VS->6:FS)

4->6: look at 4-5, 5-6
                    
T_i,j 
```

```rust
pub struct AllocationCreateDesc<'a> {
    /// Name of the allocation, for tracking and debugging purposes
    pub name: &'a str,
    /// Vulkan memory requirements for an allocation
    pub requirements: vk::MemoryRequirements,
    /// Location where the memory allocation should be stored
    pub location: MemoryLocation,
    /// If the resource is linear (buffer / linear texture) or a regular (tiled) texture.
    pub linear: bool,
    /// Whether the memory must be available immediately.
    pub no_wait: bool,
}

pub struct Allocation {
    offset: u64,
    size: u64,
    memory_block_index: usize,
    memory_type_index: usize,
    device_memory: vk::DeviceMemory,
    mapped_ptr: Option<std::ptr::NonNull<std::ffi::c_void>>,
    wait_source: QueueSerialNumbers,
    wait_src_stages: [vk::PipelineStageFlags; MAX_QUEUES] 
}

impl Allocator {
    pub fn allocate(&mut self, current_serials: QueueSerialNumbers, desc: &AllocationCreateDesc) -> Result<Allocation>
}
```

## Decision for memory aliasing
The gist: memory aliasing as it is done right now forces us to defer the allocation of resources to the end of the frame.
This prevents us from building and caching auxiliary structures such as descriptor sets and framebuffers, and overall 
increases the complexity of the mid-level renderer, who has to carry the information to build descriptor sets until command buffer creation.
Also, the current aliasing algorithm is incomplete as it doesn't take into account precise barriers between pipeline stages.

Another option: allow manual (but optionally checked) resource aliasing, and let the application derive correct aliasing
info from their own render graph.

## Rendergraph-style API: in graal or MLR?

Three features:
- autosync
- resource aliasing
- pass reordering

Any mix:
Today: 
- graal: autosync + aliasing
- MLR: pass reordering (maybe)

Proposal:
- graal: autosync
- MLR: render graph (reordering, aliasing)

Problem: 
efficient reordering and aliasing needs to consider memory dependencies between passes, so we end up building the 
same kind of structures as the graal backend.
=> what **is** this data structure? an unordered graph of passes with memory dependencies between them.


Problem: there is not a single metric to optimize for pass reordering (maximize occupancy? minimize peak memory usage?)
-> the quality of memory aliasing dependends on how passes are ordered
-> let the app decide, reorder their passes and alias memory as they see fit


Quoting webgpu about autosync:

    Inserting optimal Vulkan/D3D12 barriers at the right times appears to be a complex task, especially when taking multiple independent queues into consideration.
    It requires knowledge ahead of time on how a resource is going to be used in the future, and thus would need us to defer actual command buffer recording until we get more data on how resources are used. 
    This would add more CPU overhead to command recording.

About aliasing: https://github.com/gpuweb/gpuweb/issues/63

### Current system: autosync
Currently, graal is a simple autosync system that takes as input an *ordered* list of passes that declare the resources
that they access. They only access the last version of the resource. For each resource accessed this way, 
graal automatically syncs on the last read or write (dependending on the requested access).
Command submission must be delayed because the full sync is not known until all passes are submitted.

Rule: the backend *should not* contain pass reordering, because there is not a single metric to optimize ordering.
So pass reordering must come as a layer on top.



Implementing memory aliasing on top? 
Let's say that, at a higher level, we have a graph of draw ops (like a render graph of a compositing app). 
Each op has an output image. 
-> Determine op ordering.
-> Determine which resources to alias with.

Can we devise a more efficient autosync?

### Proposal: keep current system
- graal: autosync+aliasing 
- above: pass reordering

## Problem: descriptor sets and image views
Not only we can't create descriptor sets in advance, but we can't even write the `VkDescriptorImageInfo` in advance, 
because we must construct `VkImageViews`, and we don't have memory bound.

Solution?
* just go fcking bindless already and forget this descriptor shit
Decision? 
* we want a simple API, where we don't have to specify passes: these should be inferred from draw calls

## Tradeoffs: Render graph VS command stream
* render graph: easier on the implementation, more precise barriers, automatic aliasing, but *very poor* API
* command stream: needs rewrite, less precise barriers, limited memory aliasing, but *cleaner* API

Basically, retained vs immediate tradeoffs.

## Alternatively: don't use callbacks for command buffer recording
Two phases: 
- phase 1: create the dep graph
- inter-phase: allocate transients, figure out barriers
- phase 2: record commands

Resources must live 


# Issue: upload buffers during recording
Problem: we can upload data to buffers during recording. For example, inline uniforms contained in `ShaderArguments` impls
are uploaded when creating `ArgumentBuffers`, which can **only** be created during recording because they need memory to be 
bound to the images & buffers they reference (in order to create descriptor sets).

However, we need to add a dependency on the upload buffer when using it in a pass.
This is problematic because we can't add the dependency during the recording stage, which in turns means that:
- we must eagerly add a dependency on the upload buffer even if it's not going to be used in a pass
- OR we must require the user to add this dependency by hand => god please no
- the UPB resource must be known at pass building time, and cannot change => we can't allocate another UPB if we run out of space during recording.

Options:
- require users to put all uniforms in buffers, and upload during setup => can't have inline uniforms as part of argbuffers, **big ergonomics fail**
- do something special with UPBs:
  - manually insert pipeline barriers before commands that

=> Actually, no barrier is needed! because of [Host Write Ordering Guarantees](https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/vkspec.html#synchronization-submission-host-writes) .


# Ergonomics of the render graph API

In short: not very good compared to a straight recording process with inferred pipeline barriers. 
We need to:
1. manually split the work in passes
2. for each pass, create small accessors/descriptors/whatever during setup to access the resource safely during the pass callback.

Maybe, at some point, consider switching to wgpu for real. 
It's not possible right now because we can't really mix native vulkan code and wgpu. 
Also, there's no way of aliasing the memory between two resources, and there's an ongoing debate about a more precise
autosync (the current barriers are very conservative), and there's no multi-queue?

However:
- it's developed and used by a lot of people
- it has good docs? a good spec?

The problem with RG API is that it's difficult to have building blocks like this:
```rust
fn run_screen_pass(pipeline: Pipeline, args: &ArgumentBlock) -> ImageAny {
    // do stuff
}

fn render(frame: &Frame) {
    let args = frame.create_argument_block(Args { input: input.to_sampled_image_2d() });
    let output = run_screen_pass(pipeline, args);
}
```
because we can't infer dependencies from an argblock in this state.
So instead, the building block is something that must be wrapped manually in its own pass:
```rust
fn run_screen_pass(record: &mut RecordingContext, pipeline: Pipeline, args: &ArgumentBlock, output: ColorAttachment) {
    // do stuff
}

// problem: can't pass anything other than simple data uniforms in the provided arguments   
fn run_screen_pass_2<T: Arguments>(frame: &mut Frame, pipeline: &Pipeline, input: &ImageAny, args: Arguments, output: &ImageAny) 
{
    frame.graphics_pass(|setup| {
        let input = input.to_sampled_image_2d(setup);
        let output = output.to_color_attachment(setup);

        move |record| {
            let args = record.create_argument_block(args);
            // ...
        }
    });
}

fn render(frame: &Frame) {
    frame.graphics_pass(|setup| {
        let input = texture.to_sampled_image_2d(setup);
        let output = texture_2.to_color_attachment(setup);
        
        move |record| {
            let args = record.create_argument_block(Args { input: input.to_sampled_image_2d() });
            run_screen_pass(record, pipeline, args, output);
        }
    });
}
```

Anyway, it shouldn't be too hard to go back to the callback-based mechanism if need be.

# Pipelines and pipeline arguments
To build a pipeline we need:
- a description of the _fragment output interface_, to create the render pass
- a description of the _shader resource interface_, i.e. all _descriptor set layouts_
- a description of the _vertex input interface_, i.e. all vertex buffers

We could encode all of that into a type, like so:
```rust
#[derive(PipelineInterface)]
struct MyPipelineInterface {
    #[vertex_input] input: VertexInputInterface,
    #[arguments(set=0)] args: ArgumentBlock<BackgroundParams>,
    #[arguments(set=1)] textures: ArgumentBlock<GlobalTextures>,
    #[fragment_output] output: FragmentOutputInterface
}
```
but then we have to avoid redundant CmdBindDescriptorSets/CmdBindVertexBuffers by keeping track of the current bound blocks.

Alternative:
```rust
fn main() {
    let pipeline : GraphicsPipeline = GraphicsPipeline::builder()
        .with_arguments::<SceneArguments>(0)
        .with_arguments::<MaterialArguments>(1)
        .with_arguments::<ObjectArguments>(2)
        .with_fragment_output::<MyFragmentOutput>()
        .with_vertex_input::<MyVertexInput>().build().expect();
    // -> can already validate stuff, so that's good
    
    // note that the pipeline may not be created in advance:
    // - the vertex input can be left unspecified
    // - the color attachment formats may be left unspecified
    // - specialization constants?

    // could possibly wrap it in a generic type that specifies the VertexInput and FragmentOutput, for extra type safety.
}
```

# Graal: device/context split

Currently: the _device_ owns the underlying vulkan device 
and holds the resources (buffers and images), whereas the _context_ holds frame submission state: the current SN / frame number, 
in flight passes, etc.
However, the split does not make a lot of sense considering that the context stores tracking information directly in
resources, inside the device, so there's a bit of context state inside the device. Also it doesn't make sense to have multiple contexts
on one device.

This split is useful, however, to differentiate resource management from command submission. A separate context object
can be borrowed mutably (e.g. when building a frame) without locking the device used to allocate resources => thus 
you have only one interface to allocate resources, available at any time.

If there was only one object, the context:

```rust
fn main() {
    let ctx = Context::new();
    
    // does not borrow context
    let image = ctx.create_image();
    
    // does not borrow, because otherwise it would be impossible to allocate resources
    let mut frame = ctx.start_frame();
    
    // the following is invalid, will panic
    // ctx.start_frame();
    
    let another_image = ctx.create_image();
    
    // holds a
    frame.();
    
    ctx.finish_frame(frame);
    
}
```


mlr::Context owns the Context
mlr::Frame owns Frame, which borrows from Context, and thus borrows from mlr::Context


## Context-passing without TLS:
```rust
#![feature(fn_traits)]
#![feature(unboxed_closures)]

pub struct Cx {
}

trait CxFnOnce<'a, Args> {
    type Output;
    fn cx_call_once(self, cx: &'a mut Cx, args: Args) -> Self::Output;
}

trait CxFnMut<'a, Args>: CxFnOnce<'a, Args> {
    fn cx_call_mut(&mut self, cx: &'a mut Cx, args: Args) -> Self::Output;
}

trait CxFn<'a, Args>: CxFnMut<'a, Args> {
    fn cx_call(&self, cx: &'a mut Cx, args: Args) -> Self::Output;
}


impl<'a, F, Args> CxFnOnce<'a, Args> for F where F: FnOnce<Args> {
    type Output = F::Output;
    fn cx_call_once(self, cx: &'a mut Cx, args: Args) -> Self::Output {
        self.call_once(args)
    }
}


fn test() {
}

/*fn cx_test(node: &Node) -> Node {
    
}*/

struct Node;

struct cx_test;
impl<'cx, 'a> CxFnOnce<'cx, (&'a Node,)> for cx_test {
    type Output = Node;
    fn cx_call_once(self, cx: &mut Cx, args: (&'a Node,)) -> Self::Output {
        let (node,) = args; 
        // function body
        Node
    } 
}

struct CxFnWrapper<F>(F);

impl<'cx, F, A0> CxFnOnce<'cx, (A0,)> for CxFnWrapper<F> where F: FnOnce<(&'cx mut Cx, A0)> {
    type Output = <F as FnOnce<(&'cx mut Cx, A0)>>::Output;
    fn cx_call_once(self, cx: &'cx mut Cx, args: (A0,)) -> Self::Output {
        let (a0,) = args; 
        self.0.call_once((cx, a0))
    } 
}

struct GraphicsView;

impl GraphicsView {
    pub const new: CxFnWrapper<fn (&mut Cx, i32) -> GraphicsView> = CxFnWrapper(GraphicsView::__new);
    
    pub fn __new(cx: &mut Cx, whatever: i32) -> GraphicsView {
        GraphicsView
    } 
}

/*impl GraphicsView {
    #[composable]
    pub fn new() -> GraphicsView {
        
    }
}*/



fn main() {

    let mut cx = Cx{};
    test.cx_call_once(&mut cx, ());
    //cx_test.cx_call_once(&mut cx, (&Node,));
    
    <_>::cx_call_once(cx_test, &mut cx, (&Node,));
    <_>::cx_call_once(GraphicsView::new, &mut cx, (42,));
    
}
```

Proc-macro rewrites all calls to `<_>::cx_call_once`.
Problem with methods: we lose autoref. Not a big issue, we don't support methods right now anyway.

Upsides:
- cannot accidentally forget the `composable` attribute
- no need for TLS
- plays well with the borrow checker

Downsides:
- rewriting all function calls might be costly in terms of compilation time
- a bit of a hack
- (deal-breaker) what about function calls inside macros?
  - `let widget = dbg!(widget())`
  - no "eager expansion" currently, so this is basically a deal-breaker

# Debugging kyute

There should be a way to "tag" cache entries to see which part of the source code they map to.
E.g. is it a memoization entry? is it a state slot? where was it emitted in the call tree?

# TODOs for kyute
- not sure that the `debug_name` parameters are useful (can just click on the file link to see the relevant lines in context)
- 

# Closures for event handling

Currently, events are handled during recomposition (events invalidate state entries that cause recomp).
Can produce a lot of recomps.
E.g. with buttons, if we add `hovered`, `clicked`, `pressed`, `released`, we will trigger a recomp on every
Could also imagine a `Canvas` widget for which we have access to the current mouse position: in this case, it will recomp on **every** mouse move,
even if the user doesn't use the state.

Instead, could use closures:
- closures that change state during event delivery 
   - state must be wrapped in Arc<RefCell>, but that's probably already the case.
   - issue: clone-capture in closures is super annoying, will need a macro, and lose autocompletion in the process 
- then, recomposes because the state has changed
- less recompositions, no need for `#[uncached]`

Could be considered, but not an implementation priority. The current workflow works, even if it's a bit weird and/or inefficient (lots of recomps).

# TODO Menus
- ensure ID counter not overflowing 16-bit
- strong type for command ID
- enabled/disabled items

# GUI styling language
- high-level description of what to draw in some abstract "region" of the canvas
  - region = rect, mostly
  - but also variables indicating the position of key elements (e.g. the knob of a slider)
- sizing & layout information
  - given the size of the contents, what will be the size of the widget

Basically CSS but with the ability to render 

e.g., a button:
```
// selector: shape + paint


{
    background: linear-gradient(90deg, rgba(...) 10%, rgba(...) 20%)
}

hover { }
[hover && !hot] linear-gradient(90deg, rgba(...) 10%, rgba(...) 20%)


[hot] border 1dp {
    url(
}

hover: border 1dp {
    linear gradient (...)
}
```

In code: a high-level, descriptive/declarative drawing interface.

Base shape + modifiers

# Call-site memoization should only store the arguments that are not statically known

Consider this call: 
```rust
#[composable]
fn test() {
    TextEdit::new(/*text*/ text, /*font family*/ "consolas", /*font size*/ 14.0)
}
```

We don't need to store and compare the font family and size at this callsite because they never change.
We know that, but how can we determine that in code?
-> probably impossible without a compiler plugin

// -> jetpack compose does this, presumably with compiler magic
// -> issue: how do we know which parameters are statically known at the call site?
//   -> we don't even have access to the call site in the macro