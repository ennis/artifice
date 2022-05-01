use crate::future::FutureExt;
use artifice::{
    eval::{
        imaging::{
            ImageInputRequest, OpImaging, OpImagingCtx, PxSize, RegionOfDefinition, RequestWindow, TiPoint, TiRect,
            TiSize,
        },
        EvalError,
    },
    model::{DocumentConnection, ModelPath, Node, Value},
};
use async_trait::async_trait;
use futures::future;
use kyute::graal;
use kyute_common::{Atom, Rect};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use tokio::task;

/// A hash that uniquely identifies an evaluation of a graph object (at a given time, etc).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EvalHash(u64);

/// Gaussian blur node.
pub struct OpBlurNode(Node);

impl OpBlurNode {}

/// Applies a gaussian blur on its input.
pub struct OpBlur;

fn main() {}
