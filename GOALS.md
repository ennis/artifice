Artifice: a code base for graphics experiments with GPU.

Current pain points when trying stuff with GPU:
- write from scratch: lots of boilerplate code
- use an engine: need to learn to use it, and constrained to what the engine provides
	- plugins? need to learn the plug-in API or rather reverse-enginner it sometimes due to poor documentation
- use an helper (API-agnostic) library: constrained to the concepts that the library provides, can't use "exotic" GPU features that are not wrapped

Also:
- interactivity/tweakability is key


- "Excel" for rendering
- Free-form experimentation

- Visualizers
	- vector fields
	- scalar fields
	- other
	- color conversions
	- Zoom/pan 
	- Save to file

- "excel"-type viewport/viewer
	- put any number of images in any layout (side by side, grid, overlay, whatever)
	- infinite canvas
	- some ops can provide custom behaviors on the viewer
		- painting
		- drawing curves
		- selection rectangles
		- picking objects

- Object database
	- VFS containing shaders / node instances / assets / images
	- access by standardized path syntax
	- shaders can refer to other shaders by path

- Node graph

	- Structured edges
		- Group related images together (G-buffers?)
			- "buses": groups of coherent images 

	- Hide useless connections
	- Focus on single node

	- enforced layouts
		- vertical/horizontal stack

	- snap everything to grid, always
	- notes/comments

	- Nodes
		- not all drawn the same
		- dots
		- for buses: splice out / splice in nodes 


- Properties panel
	- Built-in GUIs
		- Gradients
		- Curve editors

- Scene import
	- USD?

- Shaders
	- Shader templates that are recompiled based on the current node inputs
	- shader libraries => contain many entry points
	- menu to pick any compatible entry point from shader libs in the VFS

- Ops
	- blueprint for nodes
	- can add dynamic params or other inputs

- Builtin nodes:
	- blur, resampling, color conversion, quantization, vector field ops, gradient, laplacian...
	- blending

- Networks:
	- have global (ambient) params

- Undo/redo:
	- via immutable data model

- Serialization
	- sqlite 



# Internals


## Data model basic concepts:

Basic concepts:
Nodes, properties and share groups.

Nodes and Properties are NamedObjects, which have a name.
Nodes have child nodes.
Nodes have properties.
Properties have a type and a value or a share group.

Are the list of child nodes and properties ordered?
(or rather, is the order kept?)
-> No, since JSON parsers usually don't
-> For UI, store UI order separately

## Serialization

Problem: stream serialization versus share groups.
We want to serialize share groups only once, but they are referenced multiple times (one for each share).
Same with connections, etc. The object graph is a DAG, not a tree, so traversal is weird.
Same for loading: connections/share groups must be loaded after.

Alternatives:
- load/store in a database (sqlite)
- don't serialize serially (pass SerializationContext, add stuff inside)

## SQLite data backend

After the in-memory data model is update, write it back to disk, *but* only write what has changed.
How do we know what has changed? 
Need sync between database and data model => increased complexity.
What about undo/redo? do we write to the database when undoing stuff? do we rollback a transaction? what if it somehow 
becomes out-of-sync with the in-memory model? 
Many chances to get stuff wrong.

Better option: write out a diff between two revisions of the in-memory data model.
But how to diff the app data? How to diff lists?

Problem with hierarchies: the data model is a hierarchy of nodes, so loading a node (e.g. the root) means to load the 
whole file recursively.

### Data model
- Nodes & attributes
- Buses (groups of attributes)

OpBlur 	
  float standardDeviation  (doc = "Standard Deviation", min = 0.01)
  image input:image        (doc = "Image to blur")
  image output:image       (doc = "Blurred image")

The "doc" metadata is not stored on each item, that would be wasteful; instead it's stored in the schema.
Think of schemas as like a "struct declaration" in Rust; a node would be an instance of that struct (a bit more complicated
than that actually, since it would be possible to dynamically add fields).

```rust
#[derive(Schema)]
struct OpBlur {
	/// Standard deviation of the gaussian kernel. Also determines the size in pixels of the kernel.
	#[schema(name="standardDeviation", default_value=2.0, ui(min=0.1))]
	standard_deviation: Attribute<f64>,
	/// Input image.
	// `Image` is a dummy type here
	#[schema(name="input", ui(input_bus="main"))]
	input: Attribute<Image>,
}
```

Data:
```

Node (schema="OpBlur")
	f64 standard_deviation (connection = ... 

```


Buses? by convention?
Attribute metadata:
	string ui:bus


	OpBlur 	
		float standardDeviation  (doc = "Standard Deviation", min = 0.01)
		image input:image        (doc = "Image to blur", ui:inputBus = "main")
		image output:image       (doc = "Blurred image", ui:outputBus = "main")


	node[OpLoad] load_0
		string filePath = "..." `File path of the image to load. Can be a URL.`
		image output:image (ui:port = "output")
		

	node[OpLoad] load_1
		string filePath = "..."

	node[OpBlur] blur
		float standardDeviation =  2.0						`Standard deviation` (ui:min=0.1)
		image input:image       = connect </load_0.output> 	`Image to blur` (ui:inputBus = "main")
		image output:image      						    `blurred image` (ui:outputBus = "main")
		
		


Design a data model that accommodates only what's needed for the GUI: buses, connections.

### Presentation model
- Current root node
- Position in the graph
- data model

UI update: has the presentation model changed?
Yes:
Node view update:
- query root node, compare with previous version

Problem: how to compare two nodes?
Arc-equality not enough since nodes don't "own" their children anymore. 
I.e. you can modify a property in the DB without changing the Arc-equality of the node.

=> the data is not the node, but the node *and its complete list of descendants*.

However:
- must query the DB for all descendant nodes/properties + share groups to check for changes
- must store this result somewhere in a materialized form

Q: do nodes have a materialized hierarchy? (a DAG of Arc<Node>) or does this hierarchy only lives in the DB?

The main question: when do we want to update the "authoritative" data source?
- traditional: when saving the file
- DB: possibly on every operation


Web: query data from the server, convert to internal data objects, display
=> on change: send update to the server, get updated data and convert to internal data objects, display

Problem: how to rebuild only the data that changed.

UI: 
- emits a command when an update of the value of a property, or share group etc. is requested
- this command defines a subtree of affected nodes
- execute update
- now rebuild the objects of affected paths (recursive process)
	- unaffected paths are just copied over to the next version


We end up with a new object tree that we can feed to the UI.
  


## Object VFS / data model

Each object is immutable & cheaply clonable. Has a trait (`ModelObject`) to query children.
Has a generic `update` method. 

### Change listeners? Watchers?

If a portion of the data model changes, other parts may want to update themselves in return (other than the UI).
Watch a path in the object VFS for changes.

Watchers own a clone of an object, and a path. On update, retrieve the object at the path, and compare it to the stored object. 
If same, does nothing, otherwise updates itself.
=> "change propagation"

After change propagation is done, update UI.

### Errors
Change propagation may fail. If so, rollback (we stored a clone of the model before the change), and display error somehow. (add error to a data model)

### Lazy loading
Objects can be created lazily (deserialize on request from the DB). Might conflict with change listeners.

## Operators (Ops)
Implemented in rust. Interprets properties from the data model object. Called by the evaluator to get the value of a node.
Operator instances are stored next to the objects. Could be inside, as an interior mutability cell.



# Document mutations

Fact: document mutations cannot be local (restricted to the item and subtree) in general, because of share groups.
A mutation can affect any part of the document. 

## Option A: perform DB transaction inline

```rust
fn widget(node: &Node) {
	
	children_widget(node);
	let button = Button::new("Add child");
	
	if button.clicked() {
		// add a child node
		//let document = node.document();
		// acquire the exclusive document lock 
		// won't work because node.document() lifetime won't be extended
		let mut document_db = node.document().lock_db();
		// with this lock, we have exclusive access to the DB
		let mut edit = document_db.begin_edit();
		
		// alternative: lambda, but that's annoying
		
		
		// `begin_edit` creates an internal clone of the document to hold the modifications.
		edit.create_node()
		// OR: `node.add_child(edit, ...)`
		// will modify the node already, but not update the 
		
		// OR: node.begin_edit(): begin an edit rooted at this node
		// saves the current state of the node, and performs modifications on the subtree
		// edit.finish(): propagates changes via share groups, performs validation, and then commits to the DB, updates
		// the current document state.
		// Problem: in the current call stack, the widgets will see the document as it was before propagation, which might 
		// not be consistent.
		// => widgets see the document in an inconsistent state
		// => widgets shouldn't see the document in an inconsistent state
		
		// => create edit on document
		// modify doc through the edit
		// finish edit
		
	}
	
}
```


## Option B: commands

Send a command up the widget tree, top-level handler handles the command and modifies the document.

```rust
fn widget(node: &Node) -> Widget {
	
	children_widget(node);
	let button = Button::new("Add child");
	
	if button.clicked() {
		// add a child node
		// what does 'sending a command' means in kyute? not much
		
	}
	
}
```




## Option C: pass down a mut ref to the database


```rust
// specify `uncached` for arguments that shouldn't be taken into account for caching (mutations?)
#[composable]
fn widget(#[uncached] document: &mut DocumentDb, node: &Node) -> Widget {
	
	children_widget(node);
	let button = Button::new("Add child");
	
	if button.clicked() {
		// add a child node
		
		let edit = document.begin_edit();
		// do stuff
		edit.finish();
		
	}
	
}
```


Try option C.

## Node schemas
What are they for?
- currently, only for operators to get their parameters

## Can schemas be static?
It would be nice if all node schemas could be 'static (loaded at the start of the application).
Problem is that the user might want to make its own non-static schemas, or load schemas from file.

## Notes

	// TODO:
	// - obtain the node at the given path
	// -

	// Q: is it a path to a node? or to an (output) attribute? or nothing? A: it's a path to a node
	// Q: how do I get the ID of the source operator? A: it's a metadata item on the node
	//      - is it an attribute on the node?
	//      - is it "something" on the attribute? its type?
	// Q: what is the type of an attribute representing an image? it has no persistent value, so... ? A: it's a dummy, unrepresentable, unserializable type
	//      - rule: nothing is stored in the file if an attribute "has no value"
	//          -> only store data?
	//      problem: the UI needs to query the operator to obtain unconnected inputs
	// Q: how to access the document from ImagingEvalCtx? A: via the parent GeneralCtx
	// Q: what about multiple outputs? A: yes
	//
	// Decisions:
	//  - outputs are represented in the datamodel by valueless attributes of type image
	//  - there can be multiple output images (called "planes"), however:
	//      - they all share the same RoD
	//      - they
	//
	//
	// Decisions:
	// - attributes may not have a defined value, because some attributes represent value that only make sense in specific contexts (i.e. they have a variability that
	// - attributes may be "connected" to other attributes, signalling that they should use the connected attribute's value instead