use crate::document::{Document, DocumentId};
use crate::util::MessageBus;
use druid_shell::{RunLoop, WindowBuilder};
use slotmap::SlotMap;
use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use typemap::TypeMap;

slotmap::new_key_type! {
    pub struct WindowId;
}

pub trait Component: Any {}

struct ComponentWrapper<C: Component>(PhantomData<C>);

impl<T: Component> typemap::Key for ComponentWrapper<T> {
    type Value = Rc<RefCell<T>>;
}

/// Root application object. Contains the application message bus and the application components.
pub struct Application {
    bus: MessageBus,
    components: TypeMap,
}

impl Application {
    pub fn new() -> Application {
        druid_shell::Application::init();
        Application {
            components: TypeMap::new(),
            bus: MessageBus::new(),
        }
    }

    /// Gets an existing component.
    pub fn component<C: Component>(&self) -> Option<Rc<RefCell<C>>> {
        self.components.get::<ComponentWrapper<C>>().cloned()
    }

    /// Adds a new component.
    pub fn add_component<C: Component>(&mut self, component: Rc<RefCell<C>>) {
        self.components.insert::<ComponentWrapper<C>>(component);
    }

    /// Returns the application message bus.
    pub fn message_bus(&self) -> &MessageBus {
        &self.bus
    }
}
