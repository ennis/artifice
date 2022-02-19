//! GUI positional cache.
use crate::{
    application::ExtEvent,
    cache,
    call_key::{CallId, CallIdStack, CallNode},
    composable_context, Data, Environment,
};
use kyute_macros::composition_context;
use kyute_shell::winit::event_loop::EventLoopProxy;
use slotmap::SlotMap;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashSet,
    convert::TryInto,
    fmt,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    mem,
    panic::Location,
    rc::Rc,
    task::Poll,
};
use thiserror::Error;
use tracing::{trace, warn};

slotmap::new_key_type! {
    struct CacheEntryKey;
}

/// Entry representing a mutable state slot inside a composition cache.
struct StateEntry {
    call_id: CallId,
    /// For debugging purposes.
    call_node: Option<Rc<CallNode>>,
    /// Whether the value has been invalidated because a dependency has changed.
    dirty: Cell<bool>,
    dependents: HashSet<CacheEntryKey>,
    value: Box<dyn Any>,
}

/// A slot in the slot table.
enum Slot {
    /// Marks the start of a group.
    /// Contains the length of the group including this slot and the `GroupEnd` marker.
    StartGroup {
        call_id: CallId,
        len: u32,
    },
    /// Marks the end of a scope.
    EndGroup,
    Value {
        call_id: CallId,
        key: CacheEntryKey,
    },
}

/// A key used to access a state variable stored in a `Cache`.
// TODO right now the get/set methods on `Key` can only be called in a composition context,
// but not outside. To set the value of a cache entry outside of a composition context,
// we have to call `cache.set_state(key)`.
// A possible change would be to keep some kind of weak ref to the cache inside the key,
// and allow calling `Key::set` outside of a composition context.
// This way there would be a single function to call, regardless of the calling context, streamlining the
// API.
// Counterpoint: this bloats the struct with an Arc pointer.
// Counter-counterpoint: but at least this makes the whole system self-contained.
pub struct Key<T> {
    key: CacheEntryKey,
    // TODO: `cache: Arc<Cache>`. When setting values, schedule a recomp on the main event loop. Cache gets an EventLoopProxy on construction to send recomp events.
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Copy for Key<T> {}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        Key {
            key: self.key,
            _phantom: Default::default(),
        }
    }
}

impl<T> fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.key, f)
    }
}

impl<T: Data + 'static> Key<T> {
    /// Returns the value of the cache entry and replaces it by the default value.
    #[composable_context]
    pub fn update(&self, new_value: T) -> T {
        let cx = composition_context!();
        let prev_value = cx.cache_writer.replace_value(*self, new_value.clone());
        if !prev_value.same(&new_value) {
            cx.cache_writer.invalidate_dependents(*self);
        }
        prev_value
    }
}

impl<T: 'static> Key<T> {
    ///
    fn from_entry_key(key: CacheEntryKey) -> Key<T> {
        Key {
            key,
            _phantom: PhantomData,
        }
    }

    /// Returns the value of the cache entry and replaces it by the default value.
    /// Always invalidates.
    #[composable_context]
    pub fn replace(&self, new_value: T) -> T {
        let cx = composition_context!();
        let prev_value = cx.cache_writer.replace_value(*self, new_value);
        cx.cache_writer.invalidate_dependents(*self);
        prev_value
    }

    #[composable_context]
    pub fn set(&self, new_value: T) {
        // TODO idea: log the call sites that invalidated the cache, for debugging
        // e.g. `state entry @ (call site) invalidated because of (state entries), because of manual invalidation @ (call site) OR invalidated externally`
        let cx = composition_context!();
        cx.cache_writer.set_value(*self, new_value);
        cx.cache_writer.invalidate_dependents(*self);
    }

    #[composable_context]
    pub fn set_without_invalidation(&self, new_value: T) {
        // TODO idea: log the call sites that invalidated the cache, for debugging
        // e.g. `state entry @ (call site) invalidated because of (state entries), because of manual invalidation @ (call site) OR invalidated externally`
        let cx = composition_context!();
        cx.cache_writer.set_value(*self, new_value);
    }
}

impl<T: Clone + 'static> Key<T> {
    #[composable_context]
    pub fn get(&self) -> T {
        let cx = composition_context!();
        cx.cache_writer.get_value(*self)
    }
}

impl<T: Default + 'static> Key<T> {
    /// Returns the value of the cache entry and replaces it by the default value.
    #[composable_context]
    pub fn take(&self) -> T {
        self.replace(composition_context!(), T::default())
    }
}

/// Composition cache. Contains the recorded call tree and state entries.
struct CacheInner {
    /// The call tree, represented as an array of slots.
    slots: Vec<Slot>,
    ///
    entries: SlotMap<CacheEntryKey, StateEntry>,
    /// The number of times `Cache::run` has been called.
    revision: usize,
}

impl CacheInner {
    pub fn new() -> CacheInner {
        CacheInner {
            slots: vec![
                Slot::StartGroup {
                    call_id: CallId(0),
                    len: 2,
                },
                Slot::EndGroup,
            ],
            revision: 0,
            entries: SlotMap::with_key(),
        }
    }

    /// Sets the value of a state entry and invalidates all dependent entries.
    pub fn set_state<T: 'static>(&mut self, key: Key<T>, new_value: T) {
        if !self.entries.contains_key(key.key) {
            warn!("set_state: entry deleted: {:?}", key.key);
            return;
        }

        let value = self.entries[key.key]
            .value
            .downcast_mut::<T>()
            .expect("type mismatch");
        *value = new_value;
        self.invalidate_dependents(key.key);
    }

    fn invalidate_dependents(&self, entry_key: CacheEntryKey) {
        for &d in self.entries[entry_key].dependents.iter() {
            self.invalidate_dependents_recursive(d);
        }
    }

    fn invalidate_dependents_recursive(&self, entry_key: CacheEntryKey) {
        assert!(
            self.entries.contains_key(entry_key),
            "invalidate_dependents_recursive: no such entry"
        );
        /*if !self.entries.contains_key(entry) {
            tracing::warn!("invalidate_dependents_recursive: no such entry");
            return;
        }*/
        let entry = &self.entries[entry_key];
        //trace!("invalidate_dependents_recursive: {:?} node={:#?}", entry_key, entry.call_node);
        //if !entry.dirty.replace(true) {
        entry.dirty.set(true);
        for &d in entry.dependents.iter() {
            self.invalidate_dependents_recursive(d);
        }
        //}
    }

    pub fn dump(&self, current_position: usize) {
        for (i, s) in self.slots.iter().enumerate() {
            if i == current_position {
                eprint!("* ");
            } else {
                eprint!("  ");
            }
            match s {
                Slot::StartGroup { call_id, len } => {
                    eprintln!(
                        "{:3} StartGroup call_id={:?} len={} (end={})",
                        i,
                        call_id,
                        *len,
                        i + *len as usize - 1,
                    )
                }
                Slot::EndGroup => {
                    eprintln!("{:3} EndGroup", i)
                }
                Slot::Value { call_id, key } => {
                    let entry = &self.entries[*key];
                    eprintln!(
                        "{:3} Value      call_id={:?} key={:?} dirty={:?}",
                        i,
                        call_id,
                        key,
                        entry.dirty.get()
                    )
                }
            }
        }
    }
}

struct CacheEntryInsertResult<T> {
    key: Key<T>,
    dirty: bool,
    inserted: bool,
}

/// Holds the state during cache updates (`Cache::run`).
struct CacheWriter {
    /// The cache being updated
    cache: CacheInner,
    /// Current position in the slot table (`self.cache.slots`)
    pos: usize,
    /// Stack of group start positions.
    /// The top element is the start of the current group.
    group_stack: Vec<usize>,
    /// Stack of state entries.
    state_stack: Vec<CacheEntryKey>,
}

impl CacheWriter {
    fn new(cache: CacheInner) -> CacheWriter {
        let mut writer = CacheWriter {
            cache,
            pos: 0,
            group_stack: vec![],
            state_stack: vec![],
        };
        writer.start_group(CallId(0));
        writer
    }

    fn start_state<T>(&mut self, key: Key<T>) {
        self.state_stack.push(key.key);
        self.cache.entries[key.key].dirty.set(false);
    }

    fn end_state(&mut self) {
        self.state_stack.pop().expect("unbalanced state scopes");
    }

    /// Finishes writing to the cache, returns the updated cache object.
    pub fn finish(mut self) -> CacheInner {
        self.end_group();
        assert!(self.group_stack.is_empty(), "unbalanced groups");
        assert_eq!(self.pos, self.cache.slots.len());
        self.cache
    }

    /// Finds a slot with the specified key in the current group, starting from the current position.
    ///
    /// # Return value
    ///
    /// The position of the matching slot in the table, or None.
    fn find_call_id(&self, call_id: CallId) -> Option<usize> {
        let mut i = self.pos;
        let slots = &self.cache.slots[..];

        while i < self.cache.slots.len() {
            match slots[i] {
                Slot::StartGroup {
                    call_id: this_call_id,
                    len,
                    ..
                } => {
                    if this_call_id == call_id {
                        return Some(i);
                    }
                    i += len as usize;
                }
                Slot::Value {
                    call_id: this_call_id,
                    ..
                } if this_call_id == call_id => {
                    return Some(i);
                }
                Slot::EndGroup => {
                    // reached the end of the current group
                    return None;
                }
                _ => {
                    i += 1;
                }
            }
        }

        // no slot found
        None
    }

    fn rotate_in_current_position(&mut self, pos: usize) {
        assert!(pos >= self.pos);
        let group_end_pos = self.group_end_position();
        assert!(pos <= group_end_pos);
        self.cache.slots[self.pos..group_end_pos].rotate_left(pos - self.pos);
    }

    /// TODO docs
    fn sync(&mut self, call_id: CallId) -> bool {
        let pos = self.find_call_id(call_id);
        match pos {
            Some(pos) => {
                // move slots in position
                self.rotate_in_current_position(pos);
                true
            }
            None => false,
        }
    }

    pub fn start_group(&mut self, call_id: CallId) {
        //let parent = self.parent_entry_key();
        if self.sync(call_id) {
            /*match self.cache.slots[self.pos] {
                Slot::StartGroup { .. } => {
                    // reset the dirty flag now:
                    // if something sets it again when inside the group, it means we should run the group again
                    //mem::replace(&mut self.cache.entries[key].dirty, false)
                }
                _ => panic!("unexpected slot type"),
            }*/
        } else {
            // insert new group - start and end markers
            self.cache.slots.insert(
                self.pos,
                Slot::StartGroup {
                    call_id,
                    len: 2, // 2 = initial length of group (start+end slots)
                },
            );
            self.cache.slots.insert(self.pos + 1, Slot::EndGroup);
        };

        // enter group
        self.group_stack.push(self.pos);
        self.pos += 1;
    }

    fn dump(&self) {
        eprintln!("position : {}", self.pos);
        eprintln!("stack    : {:?}", self.group_stack);
        eprintln!("slots:");
        self.cache.dump(self.pos);
    }

    fn group_end_position(&self) -> usize {
        let mut i = self.pos;

        while i < self.cache.slots.len() {
            match self.cache.slots[i] {
                Slot::EndGroup => break,
                Slot::StartGroup { len, .. } => {
                    i += len as usize;
                }
                _ => i += 1,
            }
        }

        i
    }

    pub fn end_group(&mut self) {
        // all remaining slots in the group are now considered dead in this revision:
        // - find position of group end marker
        let group_end_pos = self.group_end_position();

        // remove the extra slots, and associated entries
        for slot in self.cache.slots.drain(self.pos..group_end_pos) {
            match slot {
                Slot::Value { key, .. } => {
                    let entry = self.cache.entries.get(key).expect("cache entry not found");
                    trace!("removing cache entry cache_key={:?} depends={:?} call_id={:?}, call_node={:#?}", key, entry.dependents, entry.call_id, entry.call_node);
                    self.cache.entries.remove(key).unwrap();
                }
                _ => {}
            }
        }

        // skip GroupEnd marker
        self.pos += 1;
        // update group length
        let group_start_pos = self.group_stack.pop().expect("unbalanced groups");
        match self.cache.slots[group_start_pos] {
            Slot::StartGroup { ref mut len, .. } => {
                *len = (self.pos - group_start_pos).try_into().unwrap();
            }
            _ => {
                panic!("expected group start")
            }
        }
    }

    /// Skips the next entry or the next group.
    pub fn skip(&mut self) {
        match self.cache.slots[self.pos] {
            Slot::StartGroup { len, .. } => {
                self.pos += len as usize;
            }
            Slot::Value { .. } => {
                self.pos += 1;
            }
            Slot::EndGroup => {
                // nothing to skip
            }
        }
    }

    fn skip_until_end_of_group(&mut self) {
        while !matches!(self.cache.slots[self.pos], Slot::EndGroup) {
            self.skip()
        }
    }

    /// Inserts a new state entry.
    ///
    fn insert_entry<T: 'static>(
        &mut self,
        call_id: CallId,
        initial_value: T,
        call_node: Option<Rc<CallNode>>,
    ) -> Key<T> {
        let key = self.cache.entries.insert(StateEntry {
            call_id,
            call_node,
            dependents: HashSet::new(),
            dirty: Cell::new(false),
            value: Box::new(initial_value),
        });
        self.cache
            .slots
            .insert(self.pos, Slot::Value { call_id, key });
        Key::from_entry_key(key)
    }

    /// Sets the value of a cache entry.
    fn set_value<T: 'static>(&mut self, cache_key: Key<T>, new_value: T) {
        let entry = &mut self.cache.entries[cache_key.key];
        *entry.value.downcast_mut::<T>().expect("type mismatch") = new_value;
    }

    fn invalidate_dependents<T: 'static>(&mut self, cache_key: Key<T>) {
        self.cache.invalidate_dependents(cache_key.key);
    }

    fn get_or_insert_entry<T: 'static, Init: FnOnce() -> T>(
        &mut self,
        call_id: CallId,
        call_node: Option<Rc<CallNode>>,
        init: Init,
    ) -> CacheEntryInsertResult<T> {
        let result = if self.sync(call_id) {
            match self.cache.slots[self.pos] {
                Slot::Value { key: entry_key, .. } => CacheEntryInsertResult {
                    key: Key::from_entry_key(entry_key),
                    dirty: self.cache.entries[entry_key].dirty.get(),
                    inserted: false,
                },
                _ => panic!("unexpected entry type"),
            }
        } else {
            let key = self.insert_entry(call_id, init(), call_node);
            CacheEntryInsertResult {
                key,
                dirty: false,
                inserted: true,
            }
        };
        self.pos += 1;
        result
    }

    fn get_value<T: Clone + 'static>(&mut self, key: Key<T>) -> T {
        let entry = &mut self.cache.entries[key.key];
        if let Some(parent_state) = self.state_stack.last().cloned() {
            //trace!("{:?} made dependent of {:?}", parent_state, key.key);
            entry.dependents.insert(parent_state);
        }
        entry
            .value
            .downcast_ref::<T>()
            .expect("unexpected type")
            .clone()
    }

    fn replace_value<T: 'static>(&mut self, key: Key<T>, new_value: T) -> T {
        let entry = &mut self.cache.entries[key.key];
        if let Some(parent_state) = self.state_stack.last().cloned() {
            //trace!("{:?} made dependent of {:?}", parent_state, key.key);
            entry.dependents.insert(parent_state);
        }
        mem::replace(
            entry.value.downcast_mut::<T>().expect("unexpected type"),
            new_value,
        )
    }

    ///
    fn compare_and_update_value<T: Data>(
        &mut self,
        call_id: CallId,
        new_value: T,
        call_node: Option<Rc<CallNode>>,
    ) -> bool {
        let CacheEntryInsertResult { key, inserted, .. } =
            self.get_or_insert_entry(call_id, call_node, || new_value.clone());
        inserted || {
            let changed = !self.get_value(key).same(&new_value);
            if changed {
                self.set_value(key, new_value);
            }
            changed
        }
    }
}

///
pub struct Cache {
    cache: Option<CacheInner>,
}

impl Cache {
    /// Creates a new cache.
    pub fn new() -> Cache {
        Cache {
            cache: Some(CacheInner::new()),
        }
    }

    /// Returns the revision index (the number of times `run` has been called).
    pub fn revision(&self) -> usize {
        self.cache.as_ref().unwrap().revision
    }

    /// Runs a composable function within this cache.
    pub fn run<T>(
        &mut self,
        event_loop_proxy: EventLoopProxy<ExtEvent>,
        env: &Environment,
        function: impl Fn(&mut CompositionContext) -> T,
    ) -> T {
        // Temporarily move the cache out of self and into the CompositionContext, and move it back to self once we've finished
        // TODO This is a remnant of the previous TLS-based context, and could be replaced with a borrow.
        let mut inner = self.cache.take().unwrap();
        inner.revision += 1;

        // start writing to the cache
        let writer = CacheWriter::new(inner);

        // initialize the composition context (which contains the cache table writer and the call key stack that maintains
        // unique IDs for each cached function call).
        let mut cx = CompositionContext {
            event_loop_proxy,
            id_stack: CallIdStack::new(),
            cache_writer: writer,
            env: env.clone(),
        };

        // run the function
        let result = function(&mut cx);

        // check that calls to CallKeyStack::enter and exit are balanced
        assert!(cx.id_stack.is_empty(), "unbalanced CallKeyStack");

        // finalize cache writer and put the internals back
        self.cache.replace(cx.cache_writer.finish());

        result
    }

    /// Sets the value of the state variable identified by `key`, and invalidates all dependent variables in the cache.
    pub fn set_state<T: 'static>(&mut self, key: Key<T>, value: T) {
        self.cache.as_mut().unwrap().set_state(key, value)
    }
}

//--------------------------------------------------------------------------------------------------

/// Context passed to composable functions when evaluating them.
pub struct CompositionContext {
    event_loop_proxy: EventLoopProxy<ExtEvent>,
    id_stack: CallIdStack,
    cache_writer: CacheWriter,
    env: Environment,
}

impl CompositionContext {
    /// Enters a composition scope.
    #[track_caller]
    pub fn scoped<R>(&mut self, index: usize, f: impl FnOnce(&mut CompositionContext) -> R) -> R {
        let location = Location::caller();
        self.id_stack.enter(location, index);
        let r = f(self);
        self.id_stack.exit();
        r
    }

    /// Returns the current call identifier.
    pub fn current_call_id(&mut self) -> CallId {
        self.id_stack.current()
    }

    /// Returns the current revision.
    pub fn revision(&mut self) -> usize {
        self.cache_writer.cache.revision
    }

    /// Returns the environment.
    pub fn environment(&self) -> Environment {
        self.env.clone()
    }

    #[track_caller]
    pub fn with_environment<R>(
        &mut self,
        env: Environment,
        f: impl FnOnce(&mut CompositionContext) -> R,
    ) -> R {
        let merged = self.env.merged(env);
        let saved = mem::replace(&mut self.env, merged);
        let r = self.scoped(0, f);
        self.env = saved;
        r
    }

    #[track_caller]
    pub fn changed<T: Data>(&mut self, value: T) -> bool {
        let location = Location::caller();
        self.id_stack.enter(location, 0);
        let key = self.id_stack.current();
        let node = self.id_stack.current_call_node();
        let changed = self.cache_writer.compare_and_update_value(key, value, node);
        self.id_stack.exit();
        changed
    }

    #[track_caller]
    fn state_inner<T: 'static, Init: FnOnce() -> T>(
        &mut self,
        init: Init,
    ) -> CacheEntryInsertResult<T> {
        let location = Location::caller();
        self.id_stack.enter(location, 0);
        let call_id = self.id_stack.current();
        let node = self.id_stack.current_call_node();
        let r = self.cache_writer.get_or_insert_entry(call_id, node, init);
        self.id_stack.exit();
        r
    }

    /// TODO document
    #[track_caller]
    pub fn state<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Key<T> {
        self.state_inner(init).key
    }

    /// TODO document
    #[track_caller]
    pub fn run_async<T, Fut>(&mut self, future: Fut, restart: bool) -> Poll<T>
    where
        T: Clone + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        struct AsyncTaskEntry {
            handle: tokio::task::JoinHandle<()>,
            revision: usize,
        }

        let CacheEntryInsertResult {
            key: task_key,
            inserted,
            ..
        } = self.state_inner::<Option<AsyncTaskEntry>, _>(|| None);

        // if we requested a restart, abort the current running task
        let mut task = task_key.take(self);

        let revision = if let Some(ref mut task) = task {
            if restart {
                trace!("run_async: restarting task");
                task.handle.abort();
                task.revision += 1;
                task.revision
            } else {
                task.revision
            }
        } else {
            0
        };

        let CacheEntryInsertResult {
            key: result_key,
            inserted,
            ..
        } = self.scoped(revision, |cx| cx.state_inner(|| Poll::Pending));

        if inserted || restart {
            let el = self.event_loop_proxy.clone();

            // spawn task that will set the value
            let handle = tokio::spawn(async move {
                let result = future.await;
                // TODO I'd really like to just do `key.set(...)` regardless of whether we're in or out the cache,
                // instead of having to do weird things like this
                el.send_event(ExtEvent::Recompose {
                    cache_fn: Box::new(move |cache| {
                        cache.set_state(result_key, Poll::Ready(result));
                    }),
                });
            });

            task = Some(AsyncTaskEntry { handle, revision })
        }

        task_key.set(self, task);
        result_key.get(self)
    }

    #[track_caller]
    pub fn group<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let location = Location::caller();
        self.id_stack.enter(location, 0);
        self.cache_writer.start_group(self.id_stack.current());
        let r = f(self);
        self.cache_writer.end_group();
        self.id_stack.exit();
        r
    }

    pub fn skip_to_end_of_group(&mut self) {
        self.cache_writer.skip_until_end_of_group();
    }

    pub fn event_loop_proxy(&self) -> EventLoopProxy<ExtEvent> {
        self.event_loop_proxy.clone()
    }

    /// Memoizes the result of a function at this call site.
    #[track_caller]
    pub fn memoize<Args: Data, T: Clone + 'static>(
        &mut self,
        args: Args,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.group(move |cx| {
            let call_id = cx.id_stack.current();
            let call_node = cx.id_stack.current_call_node();
            let args_changed =
                cx.cache_writer
                    .compare_and_update_value(call_id, args, call_node.clone());
            let CacheEntryInsertResult { key, dirty, .. } =
                cx.cache_writer
                    .get_or_insert_entry(call_id, call_node.clone(), || None);
            /* if args_changed {
                trace!("memoize: recomputing because arguments have changed {:#?}", call_node);
            }
            if dirty {
                trace!("memoize: recomputing because state entry is dirty {:#?}", call_node);
            }*/

            if args_changed || dirty {
                cx.cache_writer.start_state(key);
                let result = f(cx);
                cx.cache_writer.end_state();
                cx.cache_writer.set_value(key, Some(result));
            } else {
                cx.skip_to_end_of_group();
            }

            // it's important to call `get()` in all circumstances to make the parent state entry
            // dependent on this value.
            key.get(cx).expect("memoize: no value calculated")
        })
    }

    /// Runs the function only once at the call site and caches the result (like memoize without parameters).
    /// TODO better docs
    #[track_caller]
    pub fn once<T: Clone + 'static>(&mut self, f: impl FnOnce() -> T) -> T {
        self.state(f).get(self)
    }
}

/// Dummy function to access the composition context in a #[composable]
#[composable_context]
pub fn context() -> &mut CompositionContext {
    __cx
}

#[composable_context]
pub fn current_call_id() -> CallId {
    __cx.current_call_id()
}

#[composable_context]
pub fn changed<T: Data>(value: T) -> bool {
    __cx.changed(value)
}

#[composable_context]
pub fn revision() -> usize {
    __cx.revision()
}

#[composable_context]
pub fn state<T: 'static>(init: impl FnOnce() -> T) -> Key<T> {
    __cx.state(init)
}

#[composable_context]
pub fn once<T: Clone + 'static>(f: impl FnOnce() -> T) -> T {
    __cx.once(f)
}

#[composable_context]
pub fn run_async<T, Fut>(future: Fut, restart: bool) -> Poll<T>
where
    T: Clone + Send + 'static,
    Fut: Future<Output = T> + Send + 'static,
{
    __cx.run_async(future, restart)
}

#[composable_context]
pub fn event_loop_proxy() -> EventLoopProxy<ExtEvent> {
    __cx.event_loop_proxy()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_rewrite() {
        let mut cache = CacheInner::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_group(CallId(99));
            writer.compare_and_update_value(CallId(1), 0, None);
            writer.compare_and_update_value(CallId(2), "hello world".to_string(), None);
            writer.end_group();
            cache = writer.finish();
            cache.dump(0);
        }
    }

    #[test]
    fn test_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();
        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                eprintln!(
                    " ==== Iteration {} - item {} =========================",
                    i, item
                );
                writer.start_group(CallId(item));
                writer.compare_and_update_value(CallId(100), i, None);
                writer.end_group();
                writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
            items.shuffle(&mut rng)
        }
    }

    /* #[test]
    fn test_placeholder() {
        let mut cache = CacheInner::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_group(CallId(99));
            let changed = writer.compare_and_update_value(CallId(100), 0.0f32, None);
            let CacheEntryInsertResult { key, dirty, .. } =
                writer.get_or_insert_entry(CallId(101), None, || 0.0);
            let value = writer.get_value(key);

            if !changed {
                writer.skip_until_end_of_group();
            } else {
                writer.compare_and_update_value(CallId(102), "hello world".to_string(), None);
                writer.set_value(key, 0.0);
            }

            writer.end_group();
            cache = writer.finish();
            cache.dump(0);
        }
    }*/

    #[test]
    fn test_tagged_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();
        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                eprintln!(
                    " ==== Iteration {} - item {} =========================",
                    i, item
                );
                writer.compare_and_update_value(CallId(100 + item), i, None);
            }
            //writer.dump();
            cache = writer.finish();
            cache.dump(0);
            items.shuffle(&mut rng)
        }
    }

    /*#[test]
    fn test_take_replace() {
        let mut cache = CacheInner::new();
        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            let (index, value) = writer.take_value(false, || 0);
            writer.tagged_compare_and_update_value(CallKey(0), 123);
            writer.dump();
            writer.replace_value(index, i);
            cache = writer.finish();
        }
    }*/

    #[test]
    fn test_insert_remove() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();

        #[derive(Clone, Debug, Eq, PartialEq)]
        struct Item {
            value: u64,
        }

        impl Data for Item {
            fn same(&self, other: &Self) -> bool {
                self.value == other.value
            }
        }

        impl Item {
            pub fn new(value: u64) -> Item {
                eprintln!("creating Item #{}", value);
                Item { value }
            }
        }

        impl Drop for Item {
            fn drop(&mut self) {
                eprintln!("dropping Item #{}", self.value);
            }
        }

        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..10 {
            let num_items = rng.gen_range(0..10);
            let items = (0..num_items)
                .map(|_| rng.gen_range(0..10))
                .collect::<Vec<_>>();

            eprintln!("Items: {:?}", items);

            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                //eprintln!(" ==== Iteration {} - item {} =========================", i, item);
                writer.compare_and_update_value(CallId(item), Item::new(item), None);
                //writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
        }
    }
}
