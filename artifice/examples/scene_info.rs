use crate::future::FutureExt;
use artifice::{
    eval::{
        imaging::{
            ImageInputRequest, OpImaging, OpImagingCtx, PxSize, RegionOfDefinition, RequestWindow, TiPoint, TiRect,
            TiSize,
        },
        EvalError,
    },
    model::{metadata, Document, DocumentFile, Node, Path, Value},
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

    document.edit(|document, edit| {
        let mut load = edit.node(Path::parse("/main/load").unwrap()).unwrap();
        //.metadata(metadata::OPERATOR, "OpLoad")
        load.set("input:filePath", "data/El4KUGDU0AAW64U.jpg").unwrap();
        load.define("output:output", "image").unwrap();

        let mut blur = edit.node(Path::parse("/main/blur").unwrap()).unwrap();
        //.metadata(metadata::OPERATOR, "OpBlur")
        blur.set("param:radius", 4.5).unwrap();
        blur.define("input:image", "image").unwrap();
        blur.connect("input:image", Path::parse("/main/load/output:output").unwrap())
            .unwrap();
        blur.define("output:image", "image").unwrap();
    });

    /*
     main {
       load <OpLoad> {
           input:filePath[string] = "data/El4KUGDU0AAW64U.jpg";
           output:output[image]
       }
       blur <OpBlur> {
           param:radius[f64] = 4.5;
           input:image[image] <- `/main/load/output:output`
           output:image[image]
       }
     }
    */

    eprintln!("{:?}", document);
}
