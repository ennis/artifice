//! Sliders provide a way to make a value vary linearly between two bounds by dragging a knob along
//! a line.
use crate::event::Event;
use crate::renderer::{Colors, LineSegment};
use crate::widget::frame::FrameVisual;
use crate::{Bounds, BoxConstraints, Measurements, Point, Visual, Widget, LayoutCtx, EventCtx, TypedWidget, PaintCtx, Environment, theme};
use generational_indextree::NodeId;
use kyute_shell::drawing::{Rect, Size};
use num_traits::{Float, PrimInt};
use std::any::Any;

/// Utility class representing a slider track on which a knob can move.
pub struct SliderTrack {
    track: LineSegment,
    divisions: Option<u32>,
    // in 0..1
    value: f64,
}

impl SliderTrack {
    fn new(track: LineSegment, divisions: Option<u32>, initial_value: f64) -> SliderTrack {
        SliderTrack {
            track,
            divisions,
            value: initial_value,
        }
    }

    /// Returns the current value of the slider.
    fn value(&self) -> f64 {
        self.value
    }

    /// Ignores divisions.
    fn set_value(&mut self, value: f64) {
        self.value = value.min(1.0).max(0.0);
    }

    /// Returns the value that would be set if the cursor was at the given position.
    fn value_from_position(&self, pos: Point) -> f64 {
        /*let hkw = 0.5 * get_knob_width(track_width, divisions, min_knob_width);
        // at the end of the sliders, there are two "dead zones" of width kw / 2 that
        // put the slider all the way left or right
        let x = pos.x.max(hkw).min(track_width-hkw-1.0);*/

        // project the point on the track line
        let v = self.track.end - self.track.start;
        let c = pos - self.track.start;
        let x = c.dot(v);
        let track_len = v.length();

        if let Some(div) = self.divisions {
            let div = div as f64;
            (div * x / track_len).floor() / div
        } else {
            x / track_len
        }
    }

    /// Returns the position of the knob on the track.
    fn knob_position(&self) -> Point {
        self.track.start + (self.track.end - self.track.start) * self.value
    }
}

/// Determines the position and size of the knob
fn get_knob_width(track_width: f64, divisions: Option<u32>, min_w: f64) -> f64 {
    let w = track_width;
    let kw = if let Some(div) = divisions {
        w / div as f64
    } else {
        // default knob size
        0.07 * track_width
    };
    // apply min width
    kw.max(min_w)
}

/*fn draw_slider_knob(
    ctx: &mut PaintCtx,
    size: Size,
    pos: f64,
    divisions: Option<u32>,
    theme: &Theme,
) {
    // half the height
    let min_knob_w = (0.5 * theme.button_metrics.min_height).ceil();
    let knob_w = get_knob_width(size.width, divisions, min_knob_w);

    let off = ((w - knob_w) * pos).ceil();
    let knob = Rect::new(Point::new(off, 0.0), Size::new(knob_w, h));

    // draw the knob rectangle
    let knob_brush = DEFAULT_COLORS.slider_grab.into_brush();
    ctx.fill_rectangle(knob, &knob_brush);
}*/

struct SliderVisual {
    track: SliderTrack,
    min: f64,
    max: f64,
}

impl Default for SliderVisual {
    fn default() -> Self {
        SliderVisual {
            track: SliderTrack::new(LineSegment {
                start: Point::zero(),
                end: Point::zero()
            }, None, 0.0),
            min: 0.0,
            max: 0.0
        }
    }
}

impl SliderVisual {
    fn update_position(&mut self, layout_width: f64, cursor: Point) {
        // remove padding
        let w = layout_width - 4.0; // 2px on each side
                                    //get_slider_position(track_width, cursor, self.divisions, )
    }
}

impl Visual for SliderVisual {
    fn paint(&mut self, ctx: &mut PaintCtx) {
        // draw the frame
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event) {}

    fn hit_test(&mut self, point: Point, bounds: Bounds) -> bool {
        unimplemented!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Floating-point sliders.
pub struct Slider<T> {
    value: T,
    min: T,
    max: T,
    divisions: Option<u32>,
}

impl<T: Float> TypedWidget<T> for Slider<T>
{
    type Visual = SliderVisual;

    fn key(&self) -> Option<u64> { None }

    fn layout(
        self,
        context: &mut LayoutCtx<f64>,
        previous_visual: Option<Box<SliderVisual>>,
        constraints: &BoxConstraints,
        env: Environment,
    ) -> (Box<SliderVisual>, Measurements)
    {
        // last position
        let last_value = previous_visual.map(|v| v.track.value);

        let height = env.get(theme::SliderHeight);
        let min_knob_width = env.get(theme::SliderKnobWidth);
        //let knob_height = env.get(theme::SliderKnobHeight);
        let padding = env.get(theme::SliderPadding);

        // fixed height
        let size = Size::new(
            constraints.max_width(),
            constraints.constrain_height(height),
        );

        // position the slider track inside the layout
        let inner_bounds = Bounds::new(Point::origin(), size).inflate(padding.width(), padding.height());

        // calculate knob width
        let knob_width = get_knob_width(
            inner_bounds.size.width,
            self.divisions,
            min_knob_width,
        );
        // half knob width
        let hkw = 0.5 * knob_width;
        // y-position of the slider track
        let y = 0.5 * size.height;

        // center vertically, add some padding on the sides to account for padding and half-knob size
        let slider_track = LineSegment {
            start: Point::new(inner_bounds.min_x() + hkw, y),
            end: Point::new(inner_bounds.max_x() - hkw, y)
        };

        let visual = SliderVisual {
            track: SliderTrack {
                value: last_value.unwrap_or_default(),
                track: slider_track,
                divisions: self.divisions
            },
            min: 0.0,
            max: 0.0
        };

        (Box::new(visual), Measurements {
            size,
            baseline: None
        })
    }
}

impl<T: Float> Slider<T> {
    pub fn new(value: T) -> Slider<T> {
        Slider {
            min: T::zero(),
            max: T::one(),
            divisions: None,
            value,
        }
    }

    pub fn min(mut self, min: T) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: T) -> Self {
        self.max = max;
        self
    }

    pub fn divisions(mut self, divisions: u32) -> Self {
        self.divisions = Some(divisions);
        self
    }
}
