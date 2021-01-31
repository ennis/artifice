use std::collections::HashMap;

mod lexer;

#[derive(Clone, Debug)]
pub struct Element {
    name: String,
    inline_attributes: Vec<Value>,
    attributes: HashMap<String, Value>,
    contents: Vec<Box<Element>>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Number(String),
    String(String),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
