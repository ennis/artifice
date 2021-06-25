use druid::{LifeCycle, EventCtx, PaintCtx, BoxConstraints, LifeCycleCtx, Size, LayoutCtx, Event, Env, UpdateCtx, Widget};

pub struct RenderCtx<'a> {
    pub frame: graal::Frame<'a>,
}

pub trait Renderer<T: Data> {

    fn render(&mut self, ctx: &mut RenderCtx, data: &T);
}

pub struct Widget3D<R>(R);

impl<T> Widget<T> for Widget3D<R> where R: Renderer<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        todo!()
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        todo!()
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        todo!()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        todo!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {

        // Best solution:
        // - flush pending commands to the swapchain
        // - get the swapchain handle
        // - get the current image of the swapchain
        // - create a share handle for the swapchain image
        // - create a semaphore for the swapchain image
        // - import the swapchain image to vulkan
        // - import the semaphore to vulkan
        // - signal the semaphore on the D3D side
        // - graal: wait for the semaphore
        // - graal: render on the imported image
        // - graal: free the imported image
        // - graal: signal semaphore
        // - D3D: wait semaphore
        // - free all images
        // -> unfortunately, sharing a swapchain doesn't seem to work

        // Expendient means:
        // - allocate a temporary image on the vulkan side, export shared handle
        // - render to it
        // - import shared handle on the D3D side
        // - blit to the render target


    }
}