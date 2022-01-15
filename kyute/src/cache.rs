use crate::{
    call_key::{CallId, CallIdStack, CallNode},
    Data,
};
use imbl::HashSet;
use slotmap::SlotMap;
use std::{
    any::Any, cell::Cell, convert::TryInto, fmt, hash::Hash, marker::PhantomData, mem,
    panic::Location, rc::Rc,
};
use thiserror::Error;
use tracing::trace;

slotmap::new_key_type! {
    struct CacheEntryKey;
}

/// Error related to state entries.
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("state entry not found")]
    EntryNotFound,
    #[error("no value in state entry")]
    VacantEntry,
    #[error("state entry already contains a value")]
    OccupiedEntry,
    #[error("type mismatch")]
    TypeMismatch,
}

// refactor:
// 1. groups don't emit state entries
// 2. track parent state entry in context
// 3. State entries store a vec of dependents

// TODO rename to `CacheEntry`?
/// Entry representing a group or
struct StateEntry {
    call_id: CallId,
    /// For debugging purposes.
    call_node: Option<Rc<CallNode>>,
    /// Whether the value has been invalidated because a dependency has changed.
    dirty: Cell<bool>,
    dependents: HashSet<CacheEntryKey>,
    value: Box<dyn Any>,
}

impl StateEntry {
    /*pub fn value_mut<T: 'static>(&mut self) -> Result<&mut T, CacheError> {
        self.value
            .as_mut()
            .ok_or(CacheError::VacantEntry)?
            .downcast_mut::<T>()
            .ok_or(CacheError::TypeMismatch)
    }*/

    /*pub fn take_value<T: 'static>(&mut self) -> Result<T, CacheError> {
        if let Some(v) = self.value.take() {
            if v.is::<T>() {
                Ok(*v.downcast().unwrap())
            } else {
                // put back value
                self.value = Some(v);
                Err(CacheError::TypeMismatch)
            }
        } else {
            Err(CacheError::VacantEntry)
        }
    }*/
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
pub struct Key<T> {
    key: CacheEntryKey,
    _phantom: PhantomData<*const T>,
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

impl<T: 'static> Key<T> {
    ///
    fn from_entry_key(key: CacheEntryKey) -> Key<T> {
        Key {
            key,
            _phantom: PhantomData,
        }
    }

    /// Sets the value of the cache entry and invalidates all dependents.
    pub fn set(&self, cx: &mut CacheWriterContext, value: T) {
        cx.writer.set_value(*self, value);
        cx.writer.invalidate_dependents(*self);
    }

    pub fn set_without_invalidate(&self, cx: &mut CacheWriterContext, value: T) {
        cx.writer.set_value(*self, value);
    }
}

impl<T: Clone + 'static> Key<T> {
    /// Returns the value of the cache entry.
    ///
    /// This automatically makes the current cache entry dependent on this one.
    pub fn get(&self, cx: &mut CacheWriterContext) -> T {
        cx.writer.get_value(*self)
    }
}

impl<T: Default + 'static> Key<T> {
    pub fn take(&self, cx: &mut CacheWriterContext) -> T {
        cx.writer.replace_value(*self, T::default())
    }
}

/// Positional cache.
pub struct Cache {
    /// The call tree, represented as an array of slots.
    slots: Vec<Slot>,
    ///
    entries: SlotMap<CacheEntryKey, StateEntry>,
    revision: usize,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
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
        let value = self.entries[key.key]
            .value
            .as_mut()
            .downcast_mut::<T>()
            .expect("type mismatch");
        *value = new_value;
        self.invalidate_dependents(key.key);
    }

    pub fn get_state<T: Clone + 'static>(&self, key: Key<T>) -> T {
        self.entries[key.key]
            .value
            .downcast_ref::<T>()
            .unwrap()
            .clone()
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
        //trace!("invalidate_entry_recursive: {:?} node={:#?}", entry_key, entry.call_node);
        if !entry.dirty.replace(true) {
            for &d in entry.dependents.iter() {
                self.invalidate_dependents_recursive(d);
            }
        }
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

/// Holds the state during cache updates (`Cache::run`).
struct CacheWriter<'a> {
    /// The cache being updated
    cache: &'a mut Cache,
    /// Current position in the slot table (`self.cache.slots`)
    pos: usize,
    /// Stack of group start positions.
    /// The top element is the start of the current group.
    group_stack: Vec<usize>,
    /// Stack of state entries.
    state_stack: Vec<CacheEntryKey>,
}

impl<'a> CacheWriter<'a> {
    fn new(cache: &'a mut Cache) -> CacheWriter<'a> {
        let mut writer = CacheWriter {
            cache,
            pos: 0,
            group_stack: vec![],
            state_stack: vec![],
        };
        writer.start_group(CallId(0));
        writer
    }

    /*fn parent_entry_key(&self) -> Option<CacheEntryKey> {
        if let Some(&group_start) = self.group_stack.last() {
            match self.cache.slots[group_start] {
                Slot::StartGroup { key: group_key, .. } => Some(group_key),
                _ => panic!("unexpected entry type"),
            }
        } else {
            None
        }
    }*/

    fn start_state<T>(&mut self, key: Key<T>) {
        self.state_stack.push(key.key);
        self.cache.entries[key.key].dirty.set(false);
    }

    fn end_state(&mut self) {
        self.state_stack.pop().expect("unbalanced state scopes");
    }

    /// Finishes writing to the cache, returns the updated cache object.
    pub fn finish(mut self) {
        self.end_group();
        assert!(self.group_stack.is_empty(), "unbalanced groups");
        assert_eq!(self.pos, self.cache.slots.len());
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

    /*fn parent_group_offset(&self) -> i32 {
        if let Some(&parent) = self.group_stack.last() {
            parent as i32 - self.pos as i32
        } else {
            0
        }
    }*/

    /*fn update_parent_group_offset(&mut self) {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::Tag(_) => {}
            Slot::StartGroup { parent: old_parent, .. } => {
                *old_parent = parent;
            }
            Slot::EndGroup => {}
            Slot::State(entry) => {
                entry.parent = parent;
            }
        }
    }*/

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
                    tracing::trace!("removing cache entry cache_key={:?} depends={:?} call_id={:?}, call_node={:#?}", key, entry.dependents, entry.call_id, entry.call_node);
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
        value: T,
        call_node: Option<Rc<CallNode>>,
    ) -> Key<T> {
        let key = self.cache.entries.insert(StateEntry {
            call_id,
            call_node,
            dependents: HashSet::new(),
            dirty: Cell::new(false),
            value: Box::new(value),
        });
        self.cache
            .slots
            .insert(self.pos, Slot::Value { call_id, key });
        Key::from_entry_key(key)
    }

    /// Sets the value of a cache entry. The entry can be vacant. Resets the dirty flag.
    fn set_value<T: 'static>(&mut self, cache_key: Key<T>, new_value: T) {
        let entry = &mut self.cache.entries[cache_key.key];
        *entry.value.downcast_mut::<T>().expect("type mismatch") = new_value;
    }

    fn invalidate_dependents<T: 'static>(&mut self, cache_key: Key<T>) {
        self.cache.invalidate_dependents(cache_key.key);
    }

    fn get_or_insert_entry<T: 'static, F: FnOnce() -> T>(
        &mut self,
        call_id: CallId,
        call_node: Option<Rc<CallNode>>,
        init: F,
    ) -> (Key<T>, bool) {
        let result = if self.sync(call_id) {
            match self.cache.slots[self.pos] {
                Slot::Value { key: entry_key, .. } => {
                    let dirty = self.cache.entries[entry_key].dirty.get();
                    (Key::from_entry_key(entry_key), dirty)
                }
                _ => panic!("unexpected entry type"),
            }
        } else {
            let k = self.insert_entry(call_id, init(), call_node);
            (k, false)
        };
        self.pos += 1;
        result
    }

    fn get_value<T: Clone + 'static>(&mut self, key: Key<T>) -> T {
        let entry = &mut self.cache.entries[key.key];
        if let Some(parent_state) = self.state_stack.last().cloned() {
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
            entry.dependents.insert(parent_state);
        }

        mem::replace(
            entry.value.downcast_mut::<T>().expect("unexpected type"),
            new_value,
        )
    }

    /*fn take_value<T: 'static>(&mut self, key: Key<T>) -> Option<T> {
        let entry = &mut self.cache.entries[key.key];
        if let Some(parent_state) = self.state_stack.last().cloned() {
            entry.dependents.insert(parent_state);
        }
        if let Some(v) = entry.value.take() {
            Some(*v.downcast::<T>().expect("unexpected type"))
        } else {
            None
        }
    }*/

    /*/// If the next entry is a value of type T, returns a clone of the value, otherwise inserts a vacant entry.
    /// Automatically makes the parent state entry a dependency of this state entry.
    fn get_value<T: Clone + 'static>(
        &mut self,
        call_key: CallId,
        call_node: Option<Rc<CallNode>>,
    ) -> (Option<T>, Key<T>, bool) {
        let result = if self.sync(call_key) {
            match self.cache.slots[self.pos] {
                Slot::Value { key: entry_key, .. } => {
                    let dirty = self.cache.entries[entry_key].dirty.get();
                    let value = self.cache.entries[entry_key]
                        .value_mut::<T>()
                        .unwrap()
                        .clone();
                    (Some(value), Key::from_entry_key(entry_key), dirty)
                }
                _ => panic!("unexpected entry type"),
            }
        } else {
            let k = self.insert_value(call_key, None, call_node);
            (None, k, false)
        };
        self.pos += 1;
        result
    }*/

    /*/// Same as `expect_value`, but instead of returning a clone of the value, takes the value and leaves a vacant entry.
    fn take_value<T: 'static>(
        &mut self,
        call_key: CallId,
        call_node: Option<Rc<CallNode>>,
    ) -> (Option<T>, Key<T>) {
        let result = if self.sync(call_key) {
            match self.cache.slots[self.pos] {
                Slot::Value { key: entry_key, .. } => {
                    // TODO allow vacant entries here?
                    let value = self.cache.entries[entry_key].take_value().unwrap();
                    (Some(value), Key::from_entry_key(entry_key))
                }
                _ => panic!("unexpected entry type"),
            }
        } else {
            let k = self.insert_value(call_key, None, call_node);
            (None, k)
        };
        self.pos += 1;
        result
    }*/

    /*/// If the next entry is a value of type T, returns a clone of the value, otherwise inserts a
    /// new value entry with `init` and returns a clone of this value.
    fn get_or_insert_value<T: Clone + 'static>(
        &mut self,
        key: Key<T>,
        init: impl FnOnce() -> T,
    ) -> (T, Key<T>) {
        let value = self.get_value(key);
        let v = match value {
            Some(v) => v,
            None => {
                let v = init();
                self.set_value(key, v.clone());
                v
            }
        };
        (v, key)
    }*/

    ///
    fn compare_and_update_value<T: Data>(
        &mut self,
        call_id: CallId,
        new_value: T,
        call_node: Option<Rc<CallNode>>,
    ) -> bool {
        let (key, _) = self.get_or_insert_entry(call_id, call_node, || new_value.clone());
        !self.get_value(key).same(&new_value)
    }
}

pub struct CacheWriterContext<'a> {
    id_stack: CallIdStack,
    writer: CacheWriter<'a>,
}

pub type UiCtx<'a, 'b> = &'b mut CacheWriterContext<'a>;

/// Enters a indexed scope.
/// Must be called inside `Cache::run`.
#[track_caller]
pub fn scoped<R>(
    cx: &mut CacheWriterContext,
    index: usize,
    f: impl FnOnce(&mut CacheWriterContext) -> R,
) -> R {
    let location = Location::caller();
    cx.id_stack.enter(location, index);
    let r = f(cx);
    cx.id_stack.exit();
    r
}

#[track_caller]
pub fn changed<T: Data>(cx: &mut CacheWriterContext, value: T) -> bool {
    let location = Location::caller();
    cx.id_stack.enter(location, 0);
    let key = cx.id_stack.current();
    let node = cx.id_stack.current_call_node();
    let changed = cx.writer.compare_and_update_value(key, value, node);
    cx.id_stack.exit();
    changed
}

#[track_caller]
pub fn group<R>(cx: &mut CacheWriterContext, f: impl FnOnce(&mut CacheWriterContext) -> R) -> R {
    let location = Location::caller();
    cx.id_stack.enter(location, 0);
    cx.writer.start_group(cx.id_stack.current());
    let r = f(cx);
    cx.writer.end_group();
    cx.id_stack.exit();
    r
}

/// TODO document
#[track_caller]
pub fn state<T: Clone + 'static>(cx: &mut CacheWriterContext, init: impl FnOnce() -> T) -> Key<T> {
    let location = Location::caller();
    cx.id_stack.enter(location, 0);
    let call_id = cx.id_stack.current();
    let node = cx.id_stack.current_call_node();
    let (key, dirty) = cx.writer.get_or_insert_entry(call_id, node, init);
    cx.id_stack.exit();
    key
}

/*
#[track_caller]
fn get_cached_value<T: Clone + 'static>(cx: &mut CacheWriterContext) -> (Option<T>, Key<T>, bool) {
    let location = Location::caller();
    cx.id_stack.enter(location, 0);
    let call_id = cx.id_stack.current();
    let node = cx.id_stack.current_call_node();
    let (key, dirty) = cx.writer.get_or_insert_entry::<T>(call_id, node);
    let value = cx.writer.get_value(key);
    cx.id_stack.exit();
    (value, key, dirty)
}*/

/// Memoizes the last result of a function at this call site.
#[track_caller]
pub fn memoize<Args: Data, T: Clone + 'static>(
    cx: &mut CacheWriterContext,
    args: Args,
    f: impl FnOnce(&mut CacheWriterContext) -> T,
) -> T {
    group(cx, move |cx| {
        // the shape of the call graph within this function never changes so we can reuse call ids
        let call_id = cx.id_stack.current();
        let call_node = cx.id_stack.current_call_node();
        let args_changed = cx
            .writer
            .compare_and_update_value(call_id, args, call_node.clone());
        let (cached_result, result_dirty) =
            cx.writer
                .get_or_insert_entry(call_id, call_node.clone(), || None);
        if args_changed || result_dirty {
            // state update
            cx.writer.start_state(cached_result);
            let new_result = f(cx);
            cx.writer.end_state();
            cached_result.set_without_invalidate(cx, Some(new_result.clone()));
            new_result
        } else {
            cx.writer.skip_until_end_of_group();
            cached_result
                .get(cx)
                .expect("memoize: no changes in arguments but no value calculated")
        }
    })
}

/// Returns the current call identifier.
pub fn current_call_id(cx: &mut CacheWriterContext) -> CallId {
    cx.id_stack.current()
}

impl Cache {
    /// Runs a cached function with this cache.
    pub fn run<T>(&mut self, function: impl Fn(&mut CacheWriterContext) -> T) -> T {
        // start writing to the cache
        let writer = CacheWriter::new(self);
        let mut cx = CacheWriterContext {
            id_stack: CallIdStack::new(),
            writer,
        };

        // run the function
        let result = function(&mut cx);
        assert!(cx.id_stack.is_empty(), "unbalanced CallKeyStack");

        // finalize cache writer
        cx.writer.finish();
        result
    }

    /*#[track_caller]
    fn get_value<T: Clone + 'static>() -> (Option<T>, Key<T>, bool) {
        let location = Location::caller();
        Self::with_cx(|cx| {
            cx.id_stack.enter(location, 0);
            let call_id = cx.id_stack.current();
            let node = cx.id_stack.current_call_node();
            let (key, dirty) = cx.writer.get_or_insert_entry::<T>(call_id, node);
            let value = cx.writer.get_value(key);
            cx.id_stack.exit();
            (value, key, dirty)
        })
    }*/

    /*/// Updates a state entry.
    pub fn replace_state<T: 'static>(key: Key<T>, new_value: T) {
        Self::with_cx(move |cx| {
            cx.writer.set_value(key, new_value);
            cx.writer.invalidate_dependents(key);
        })
    }

    /// Updates a state entry.
    pub fn replace_state_without_invalidation<T: 'static>(key: Key<T>, new_value: T) {
        Self::with_cx(move |cx| cx.writer.set_value(key, new_value))
    }*/

    /*#[track_caller]
    pub fn with_state<T: Data, R>(init: impl FnOnce() -> T, update: impl Fn(&mut T) -> R) -> R {
        // load the state from the cache, or reserve a slot if it's the first time we run
        let (mut value, key, _) = Self::get_value::<T>();
        let initial = value.is_none();

        let mut value = if let Some(value) = value {
            // use the existing state
            value
        } else {
            // create the initial value of the state
            init()
        };
        let old_value = value.clone();
        let r = Self::state_scope(key, || update(&mut value));

        // if the state has changed, TODO
        if initial || !old_value.same(&value) {
            // TODO: re-run update? Invalidate?
            Self::set_value(key, value);
        }

        r
    }*/
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

    #[test]
    fn test_placeholder() {
        let mut cache = CacheInner::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_group(CallId(99));
            let changed = writer.compare_and_update_value(CallId(100), 0, None);
            let (key, dirty) = writer.get_or_insert_entry::<f64>(CallId(101), None);
            let value = writer.get_value(key);

            if !changed {
                assert!(value.is_some());
                writer.skip_until_end_of_group();
            } else {
                writer.compare_and_update_value(CallId(102), "hello world".to_string(), None);
                writer.set_value(key, 0.0);
            }

            writer.end_group();
            cache = writer.finish();
            cache.dump(0);
        }
    }

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
