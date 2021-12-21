extern crate string_cache_codegen;

use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("foo::FooAtom", "foo_atom!")
        .atoms(&["foo", "bar"])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("foo_atom.rs"))
        .unwrap()
}