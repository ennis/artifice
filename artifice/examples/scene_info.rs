use crate::future::FutureExt;
use artifice::{
    eval::{
        imaging::{
            ImageInputRequest, OpImaging, OpImagingCtx, PxSize, RegionOfDefinition, RequestWindow, TiPoint, TiRect,
            TiSize,
        },
        EvalError,
    },
    model::{Document, DocumentFile, Node, Path, Value},
};
use async_trait::async_trait;
use futures::future;
use kyute::graal;
use kyute_common::{Atom, Rect};
use parking_lot::Mutex;

/// A hash that uniquely identifies an evaluation of a graph object (at a given time, etc).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EvalHash(u64);

fn main() {
    let connection = rusqlite::Connection::open("scene_info.db").expect("could not open db file");
    let mut document = DocumentFile::open(connection).unwrap();

    document.edit(|root| {
        root.get_or_create_node("main", |main| {
            main.get_or_create_node("load", |load| {
                load.get_or_create_attribute(
                    "input:filePath",
                    "string",
                    Some(Value::String("data/El4KUGDU0AAW64U.jpg".into())),
                    |_| {},
                );

                //load.attribute("input:filePath").value("data/El4KUGDU0AAW64U.jpg").define();  // or .create()

                load.get_or_create_attribute("output:output", "image", None, |_| {});
            });
            main.get_or_create_node("blur", |blur| {
                blur.get_or_create_attribute("operator", "token", Some(Value::Token("OpBlur".into())), |_| {});
                blur.get_or_create_attribute("param:radius", "f64", Some(Value::Number(4.5)), |_| {});
                blur.get_or_create_attribute("input:image", "image", None, |attr| {
                    attr.set_connection(Path::parse("/main/load/output:output"))
                });
                // blur.attribute("input:image").connect("/main/load/output:output").create();
                blur.get_or_create_attribute("output:image", "image", None, |_| {});
            });
        });
    });

    eprintln!("{:?}", document);
}
