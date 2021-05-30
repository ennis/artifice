use crate::model::Composition;


pub struct Action {
    redo: Box<dyn FnMut(&mut Composition)>,
    undo: Box<dyn FnMut(&mut Composition)>
}


pub struct Edit {
    actions: Vec<Action>,
}