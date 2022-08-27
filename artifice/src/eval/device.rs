//! Device eval state

use crate::eval::EvalError;
use kyute::{graal, shell::application::Application};
use parking_lot::Mutex;
use std::{mem, sync::Arc, time::Duration};
use tokio::task::JoinHandle;

struct DeviceEvalStateInner {
    frame: graal::Frame<'static, ()>,
    transient_images: Vec<graal::ImageId>,
    transient_buffers: Vec<graal::BufferId>,
}

impl DeviceEvalStateInner {
    fn has_pending_work(&self) -> bool {
        !(self.frame.is_empty() && self.transient_images.is_empty() && self.transient_buffers.is_empty())
    }

    fn flush(&mut self, device: Arc<graal::Device>) -> JoinHandle<()> {
        trace!("flushing device frame");
        let progress = if self.has_pending_work() {
            let mut frame = mem::take(&mut self.frame);
            // destroy transients
            for &id in self.transient_images.iter() {
                frame.destroy_image(id);
            }
            for &id in self.transient_buffers.iter() {
                frame.destroy_buffer(id);
            }
            // FIXME: get the context from somewhere else
            let mut ctx = Application::instance().lock_gpu_context();
            let result = ctx.submit_frame(&mut (), frame, &graal::SubmitInfo::default());
            result.progress
        } else {
            graal::QueueProgress::default()
        };

        let device_future = tokio::task::spawn_blocking(move || {
            device
                .wait(&progress, Duration::from_secs(5))
                .expect("failed to wait for device")
        });
        device_future
    }
}

pub(crate) struct DeviceEvalState {
    pub(crate) device: Arc<graal::Device>,
    inner: Mutex<DeviceEvalStateInner>,
}

impl DeviceEvalState {
    pub(crate) fn new(device: Arc<graal::Device>) -> DeviceEvalState {
        DeviceEvalState {
            device,
            inner: Mutex::new(DeviceEvalStateInner {
                frame: Default::default(),
                transient_images: vec![],
                transient_buffers: vec![],
            }),
        }
    }

    pub(crate) fn has_pending_work(&self) -> bool {
        self.inner.lock().has_pending_work()
    }

    pub(crate) fn flush(&self) -> JoinHandle<()> {
        trace!("flushing device frame");
        self.inner.lock().flush(self.device.clone())
    }

    pub(crate) fn create_image(
        &self,
        location: graal::MemoryLocation,
        create_info: &graal::ImageResourceCreateInfo,
    ) -> Result<graal::ImageInfo, EvalError> {
        // TODO create_image currently never fails (it asserts), but it should return errors at some point
        let image = self.device.create_image("", location, create_info);
        self.inner.lock().transient_images.push(image.id);
        Ok(image)
    }

    pub(crate) fn create_buffer(
        &self,
        location: graal::MemoryLocation,
        create_info: &graal::BufferResourceCreateInfo,
    ) -> Result<graal::BufferInfo, EvalError> {
        let buffer = self.device.create_buffer("", location, create_info);
        self.inner.lock().transient_buffers.push(buffer.id);
        Ok(buffer)
    }

    pub(crate) fn add_pass(&self, pass: graal::PassBuilder<'static, ()>) -> Result<(), EvalError> {
        self.inner.lock().frame.add_pass(pass);
        Ok(())
    }

    pub(crate) fn make_image_persistent(&self, image: graal::ImageId) {
        let mut inner = self.inner.lock();
        if let Some(p) = inner.transient_images.iter().position(|x| *x == image) {
            inner.transient_images.swap_remove(p);
        } else {
            warn!("requested to make image {image:?} persistent but it was not found in the list of transient resources (already flushed?)");
        }
    }
}
