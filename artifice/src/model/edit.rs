#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EditAction {
    Create,
    Modify,
    Remove,
}
