use std::marker::PhantomData;
use crate::util::model::Data;
use crate::util::model::Revision;
use crate::ui::common::View;
use crate::ui::common::EventCtx;
use crate::ui::common::BoxConstraints;
use crate::ui::common::Size;
use crate::ui::common::LayoutCtx;
use crate::ui::common::ViewEvent;
use crate::ui::common::PaintCtx;

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum MainAxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum CrossAxisAlignment {
    Baseline,
    Start,
    Center,
    End,
    Stretch
}

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum MainAxisSize {
    Min,
    Max,
}

/// Widget that layouts its contents in a column.
pub struct VBox<S: Data> {
    contents: Vec<Box<dyn View<S>>>,
    main_axis_alignment: MainAxisAlignment,
    cross_axis_alignment: CrossAxisAlignment,
    main_axis_size: MainAxisSize
}

impl<S: Data> VBox<S> {
    pub fn new(contents: Vec<Box<dyn View<S>>>) -> VBox<S>
    {
        VBox {
            contents,
            main_axis_alignment: MainAxisAlignment::Start,
            cross_axis_alignment: CrossAxisAlignment::Start,
            main_axis_size: MainAxisSize::Min,
        }
    }

    pub fn contents(&self) -> &V {
        &self.contents
    }

    pub fn main_axis_alignment(mut self, align: MainAxisAlignment) -> Self {
        self.main_axis_alignment = align;
        self
    }

    pub fn cross_axis_alignment(mut self, align: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = align;
        self
    }

    pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
        self.main_axis_size = size;
        self
    }
}

impl<S: Data> View<S> for VBox<S>
{
    type Action = V::Action;

    fn event(&mut self, e: &ViewEvent, ctx: &mut EventCtx<V::Action>) {
        self.contents.event(e, ctx)
    }

    fn update(&mut self, s: &Revision<S>) {
        self.contents.update(s)
    }

    fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool {
        self.contents.paint(state, ctx)
    }

    fn layout(&mut self, state: &S, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Size
    {
        let sizes = self.contents.iter_mut().map(|v| v.layout())
        unimplemented!()
    }
}