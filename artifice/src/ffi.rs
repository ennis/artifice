

type WatcherId = i32;

/// Describes what kind of modification happened to data.
enum DataChangeKind {
    /// The value of the data was changed.
    ValueChanged,
    /// The data is a collection and an element was inserted into this collection.
    ElementInserted,
    /// The data is a collection and an element was removed from this collection.
    ElementRemoved,
}

struct DataChange {
    /// The kind of change.
    ty: DataChangeKind,
    /// If the change is an insertion or a deletion, the index at which the change happened.
    index: usize,
}

type WatchCallback = extern "C" fn (userdata: *mut std::ffi::c_void, change: &Change);

#[derive(Copy,Clone,Debug)]
#[repr(C)]
struct Instance {
    // Data model + services
}

#[derive(Copy,Clone,Debug)]
#[repr(C)]
struct RawStr(usize, *mut u8);

#[derive(Copy,Clone,Debug)]
#[repr(C)]
struct NodeInput {
    name: RawStr,
}

#[derive(Copy,Clone,Debug)]
#[repr(C)]
struct NodeOutput {
    name: RawStr,
}


#[derive(Copy,Clone,Debug)]
#[repr(C)]
struct Node {

}

#[no_mangle]
pub extern "C" fn art_create_instance() -> Instance {
unimplemented!()
}

#[no_mangle]
pub extern "C" fn art_watch_node(instance: &mut Instance, id: NodeId, cb: WatchCallback) -> WatcherId {
unimplemented!()
}

#[no_mangle]
pub extern "C" fn art_delete_watcher(instance: &mut Instance, id: WatcherId) {
unimplemented!()
}

#[no_mangle]
pub extern "C" fn art_watch_network_connections(instance: &mut Instance, network: NetworkId, cb: WatchCallback) -> WatcherId {
unimplemented!()
}

/// Watches the collection of nodes in a network.
#[no_mangle]
pub extern "C" fn art_watch_network_nodes(instance: &mut Instance, network: NetworkId, cb: WatchCallback) -> WatcherId {
unimplemented!()
}

#[no_mangle]
pub extern "C" fn art_get_node_info(instance: &mut Instance, node: NodeId) -> Node {

}

