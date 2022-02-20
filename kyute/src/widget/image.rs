use crate::{
    cache,
    widget::{prelude::*, Container, Null},
};
use kyute_shell::{application::Application, drawing::ToSkia};
use std::task::Poll;
use tracing::trace;
use crate::application::ExtEvent;

#[derive(Clone)]
enum ImageContents<Placeholder> {
    Image(kyute_shell::drawing::Image),
    Placeholder(Placeholder),
}

#[derive(Clone)]
pub struct Image<Placeholder> {
    contents: ImageContents<Placeholder>,
}

impl Image<Null> {
    /// Creates an image widget that displays the image from a specified asset URI.
    #[composable]
    pub fn from_uri(cx: Cx, uri: &str) -> Image<Null> {
        let application = Application::instance();
        let image: kyute_shell::drawing::Image = application
            .asset_loader()
            .load(uri)
            .expect("failed to load image");

        Image {
            contents: ImageContents::Image(image),
        }
    }
}

#[composable]
fn watch_file_changes(cx: Cx, uri: &str) -> bool {
    let uri = uri.to_owned();
    let changed = cx.state(|| false);
    let event_loop_proxy = cx.event_loop_proxy();

    /*tokio::task::spawn(async move {
        loop {
            let application = Application::instance();
            application.asset_loader().watch_changes(&uri).await;
            event_loop_proxy.send_event(ExtEvent::Recompose { cache_fn: Box::new(|cache| cache.set_state(changed, true)) });
        }
    });*/

    changed.update(cx, false)
}

impl<Placeholder: Widget> Image<Placeholder> {
    /// Creates an image widget that loads the image at the specified URI asynchronously,
    /// and displays the image once it is loaded.
    #[composable]
    pub fn from_uri_async(cx: Cx, uri: &str, placeholder: Placeholder) -> Image<Placeholder> {
        let application = Application::instance();
        let image_future = application
            .asset_loader()
            .load_async::<kyute_shell::drawing::Image>(uri);

        //let reload = watch_file_changes(uri);

        let image = cx.run_async(
            async move {
                let image_result = image_future.await;
                trace!("Image::from_uri_async {:?}", image_result);
                image_result.ok()
            },
            false,
        );

        match image {
            Poll::Ready(Some(image)) => {
                Image {
                    contents: ImageContents::Image(image),
                }
            }
            _ => {
                Image {
                    contents: ImageContents::Placeholder(placeholder),
                }
            }
        }
    }
}

impl<Placeholder: Widget> Widget for Image<Placeholder> {
    fn event(&self, _ctx: &mut EventCtx, _event: &mut Event, _env: &Environment) {}

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements {
        match self.contents {
            ImageContents::Image(ref img) => {
                // TODO DPI
                let size_i = img.size();
                Measurements::new(Rect::new(
                    Point::origin(),
                    Size::new(size_i.width as f64, size_i.height as f64),
                ))
            }
            ImageContents::Placeholder(ref placeholder) => {
                placeholder.layout(ctx, constraints, env)
            }
        }
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        match self.contents {
            ImageContents::Image(ref img) => {
                ctx.canvas
                    .draw_image(img.to_skia(), Point::origin().to_skia(), None);
            }
            ImageContents::Placeholder(ref placeholder) => placeholder.paint(ctx, bounds, env),
        }
    }
}
