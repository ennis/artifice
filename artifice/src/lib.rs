#![feature(type_alias_impl_trait)]

// macro support
#[macro_use]
extern crate tracing;
extern crate core;
extern crate self as artifice;

use serde_json as json;

pub mod eval;
pub mod model;
pub mod operators;
pub mod util;
pub mod view;
