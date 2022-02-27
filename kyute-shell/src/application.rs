//! Windows-specific UI stuff.
use crate::backend;
use std::{
    cell::RefCell,
    ffi::OsString,
    ops::Deref,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

/// Mutex-protected and ref-counted alias to `graal::Context`.
pub type GpuContext = Arc<Mutex<graal::Context>>;

/// Encapsulates various platform-specific application services.
// all of this must be either directly Sync, or wrapped in a mutex, or wrapped in a main-thread-only wrapper.
pub(crate) struct ApplicationImpl {
    pub(crate) backend: backend::Application,
}

/// Encapsulates application-global services.
#[derive(Clone)]
pub struct Application(pub(crate) Arc<ApplicationImpl>);

thread_local! {
    /// Platform singleton. Only accessible from the main thread, hence the `thread_local`.

    // NOTE: we previously used `OnceCell` so that we could get a `&'static Platform` that lived for
    // the duration of the application, but the destructor wasn't called. This has consequences on
    // windows because the DirectX debug layers trigger panics when objects are leaked.
    // Now we use shared ownership instead, and automatically release this global reference when
    // `run` returns.
    static APPLICATION: RefCell<Option<Application>> = RefCell::new(None);
}

/// Global flag that tells whether there's an active `Application` object in `APPLICATION`.
static APPLICATION_CREATED: AtomicBool = AtomicBool::new(false);

impl Application {
    /// Initializes the global application object.
    ///
    /// The application object will be tied to this thread (the "main thread").
    pub fn new() -> anyhow::Result<Application> {
        // check that we don't already have an active platform, and acquire the global flag
        APPLICATION_CREATED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| anyhow::anyhow!("an application has already been created."))?;

        // actually create the platform
        let application = Self::new_impl();

        let application = match application {
            Err(e) => {
                // if creation failed, don't forget to release the global flag
                APPLICATION_CREATED.store(false, Ordering::Release);
                return Err(e.context("failed to create application"));
            }
            Ok(p) => p,
        };

        APPLICATION.with(|app| app.replace(Some(application.clone())));
        Ok(application)
    }

    /// Creates the application backend.
    fn new_impl() -> anyhow::Result<Application> {
        let backend = backend::Application::new()?;
        let app_impl = ApplicationImpl { backend };
        let app = Application(Arc::new(app_impl));
        Ok(app)
    }

    /// Returns the global application object that was created by a call to `init`.
    ///
    /// # Panics
    ///
    /// Panics of no platform is active, or if called outside of the main thread, which is the thread
    /// that called `Platform::new`.
    pub fn instance() -> Application {
        APPLICATION
            .with(|p| p.borrow().clone())
            .expect("either the platform instance was not initialized, or not calling from the main thread")
    }

    /// Deletes the application object and closes the associated services.
    pub fn shutdown() {
        APPLICATION.with(|p| p.replace(None));
    }

    // issue: this returns different objects before and after `run` is called.
    // bigger issue: an `&EventLoopWindowTarget` can only be retrieved from the event loop callback,
    // once the main event loop object is consumed in `run`.
    // This means that we must pass around the event loop stuff Fuck this shit already.
    //pub fn event_loop(&self) -> &EventLoopWindowTarget<()> {
    //    &self.0.event_loop
    //}

    /// Returns the `graal::Device` instance.
    pub fn gpu_device(&self) -> &Arc<graal::Device> {
        &self.0.gpu_device
    }

    /// Locks the GPU context.
    pub fn lock_gpu_context(&self) -> MutexGuard<graal::Context> {
        self.0.gpu_context.lock().unwrap()
    }

    pub fn run() {
        APPLICATION.with(|p| p.borrow().unwrap().0.backend.run())
    }
}
