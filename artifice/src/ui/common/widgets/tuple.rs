use crate::util::model::Data;
use crate::util::model::Revision;

use crate::ui::common::PaintCtx;
use crate::ui::common::EventCtx;
use crate::ui::common::View;
use crate::ui::common::ViewEvent;

/*macro_rules! impl_tuple_view_collection {
    ((0) -> T $(($idx:tt) -> $T:ident)*) => {
        impl<S: Data, T, $($T),*> ViewCollection<S> for (T,$($T),*)
            where T: View<S>,
                $($T: View<S, Action=T::Action>),*
        {
            type Action = T::Action;

            fn update(&mut self, rev: &Revision<S>) {
                self.0.update(rev);
                $(self.$idx.update(rev);)*
            }

            fn event(&mut self, ev: &ViewEvent, ctx: &mut EventCtx<Self::Action>) {
                self.0.event(ev, ctx);
                $(self.$idx.event(ev, ctx);)*
            }

            fn paint(&mut self, state: &S, ctx: &mut PaintCtx) -> bool {
                self.0.paint(state, ctx)
                $(|| self.$idx.paint(state, ctx))*
            }
        }
    };
}

impl_tuple_view_collection!((0) -> T);
impl_tuple_view_collection!((0) -> T (1) -> A);
impl_tuple_view_collection!((0) -> T (1) -> A (2) -> B);
impl_tuple_view_collection!((0) -> T (1) -> A (2) -> B (3) -> C);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D (5) -> E);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D (5) -> E (6) -> F);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D (5) -> E (6) -> F (7) -> G);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D (5) -> E (6) -> F (7) -> G
    (8) -> H);

impl_tuple_view_collection!(
    (0) -> T (1) -> A (2) -> B (3) -> C
    (4) -> D (5) -> E (6) -> F (7) -> G
    (8) -> H (9) -> I);
*/