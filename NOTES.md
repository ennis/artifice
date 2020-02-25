
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