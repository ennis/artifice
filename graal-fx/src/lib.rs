use ash::vk;
use lalrpop_util::lalrpop_mod;
use std::collections::HashMap;
use thiserror::Error;
use strum::EnumString;

mod lexer;
lalrpop_mod!(
    #[allow(dead_code, unused_imports)]
    grammar
);

use crate::lexer::{LexerAdapter, LexicalError};
pub use bumpalo::Bump as Arena;
use std::str::FromStr;
use crate::BinaryOp::Sub;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("lexical error")]
    LexicalError(#[from] lexer::LexicalError),
}

#[derive(Copy, Clone, Debug)]
pub struct Module<'ast> {
    items: &'ast [Item<'ast>],
}

impl<'ast> Module<'ast> {
    pub fn parse<'input>(
        input: &'input str,
        arena: &'ast Arena,
    ) -> Result<Module<'ast>, lalrpop_util::ParseError<usize, lexer::Token<'input>, ParseError>>
    {
        let mut state = ParseState::new(arena);
        let mut lex = LexerAdapter::new(input);
        grammar::ModuleParser::new().parse(&input, &mut state, lex)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Item<'ast> {
    Variable(Variable<'ast>),
    Constant(Constant<'ast>),
    RenderPass(RenderPass<'ast>),
}

#[derive(Copy, Clone, Debug)]
pub struct Variable<'ast> {
    name: &'ast str,
    ty: Type,
    initializer: Option<&'ast Expr<'ast>>,
}

#[derive(Copy, Clone, Debug)]
pub struct Constant<'ast> {
    name: &'ast str,
}

#[derive(Copy, Clone, Debug)]
pub struct RenderPass<'ast> {
    name: &'ast str,
    attachments: &'ast [Attachment<'ast>],
    subpasses: &'ast [Subpass<'ast>],
}

impl<'ast> Default for RenderPass<'ast> {
    fn default() -> Self {
        RenderPass {
            name: "",
            attachments: &[],
            subpasses: &[]
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Attachment<'ast> {
    name: &'ast str,
    flags: Option<&'ast Expr<'ast>>,
    format: Option<&'ast Expr<'ast>>,
    samples: Option<&'ast Expr<'ast>>,
    load_op: Option<&'ast Expr<'ast>>,
    store_op: Option<&'ast Expr<'ast>>,
    stencil_load_op: Option<&'ast Expr<'ast>>,
    stencil_store_op: Option<&'ast Expr<'ast>>,
    initial_layout: Option<&'ast Expr<'ast>>,
    final_layout: Option<&'ast Expr<'ast>>,
}

impl<'a> Default for Attachment<'a> {
    fn default() -> Self {
        Attachment {
            name: "",
            flags: None,
            format: None,
            samples: None,
            load_op: None,
            store_op: None,
            stencil_load_op: None,
            stencil_store_op: None,
            initial_layout: None,
            final_layout: None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Subpass<'ast> {
    name: &'ast str,
}

impl<'a> Default for Subpass<'a> {
    fn default() -> Self {
        Subpass {
            name: ""
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Field<'ast> {
    name: &'ast str,
    initializer: &'ast Expr<'ast>,
}

#[derive(Copy, Clone, Debug)]
pub enum RenderPassItem {
    Attachment,
    Subpass,
}

#[derive(Copy, Clone, Debug)]
pub enum Value<'ast> {
    Number(&'ast str),
    String(&'ast str),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Add,
    Minus,
    Not,
    Complement,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    RShift,
    LShift,
    And,
    Or,
    Xor,
    Eq,
    NEq,
    Gt,
    Lt,
    GEq,
    LEq,
    LogicalAnd,
    LogicalOr,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Type {
    U32,
    I32,
    F32,
    U32x2,
    U32x3,
    U32x4,
    I32x2,
    I32x3,
    I32x4,
    F32x2,
    F32x3,
    F32x4,
}

#[derive(Copy, Clone, Debug)]
pub enum VecExpr<'ast> {
    U32x2(&'ast Expr<'ast>, &'ast Expr<'ast>),
    U32x3(&'ast Expr<'ast>, &'ast Expr<'ast>, &'ast Expr<'ast>),
    U32x4(
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
    ),
    I32x2(&'ast Expr<'ast>, &'ast Expr<'ast>),
    I32x3(&'ast Expr<'ast>, &'ast Expr<'ast>, &'ast Expr<'ast>),
    I32x4(
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
    ),
    F32x2(&'ast Expr<'ast>, &'ast Expr<'ast>),
    F32x3(&'ast Expr<'ast>, &'ast Expr<'ast>, &'ast Expr<'ast>),
    F32x4(
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
        &'ast Expr<'ast>,
    ),
}

#[derive(Copy, Clone, Debug)]
pub enum Expr<'ast> {
    /// Reference to a variable
    Variable(&'ast str),
    /// Integer literal
    IntLiteral(i64),
    /// Floating point literal
    FloatLiteral(f64),
    /// Boolean literal (true or false)
    BoolLiteral(bool),
    Unary(UnaryOp, &'ast Expr<'ast>),
    Binary(BinaryOp, &'ast Expr<'ast>, &'ast Expr<'ast>),
    /// if { then_expr } else { else_branch }
    Conditional {
        condition: &'ast Expr<'ast>,
        then_expr: &'ast Expr<'ast>,
        else_expr: Option<&'ast Expr<'ast>>,
    },
    /// vector expressions
    VecExpr(VecExpr<'ast>),
}

#[derive(EnumString)]
pub enum RenderPassPropertyKind {
    #[strum(serialize = "name")]
    Name,
    #[strum(serialize = "flags")]
    Flags,
    #[strum(serialize = "format")]
    Format,
    #[strum(serialize = "samples")]
    Samples,
    #[strum(serialize = "loadOp")]
    LoadOp,
    #[strum(serialize = "storeOp")]
    StoreOp,
    #[strum(serialize = "stencilLoadOp")]
    StencilLoadOp,
    #[strum(serialize = "stencilStoreOp")]
    StencilStoreOp,
    #[strum(serialize = "initialLayout")]
    InitialLayout,
    #[strum(serialize = "finalLayout")]
    FinalLayout,
}

impl RenderPassPropertyKind {
    pub fn parse(s: &str) -> Option<RenderPassPropertyKind> {
        FromStr::from_str(s).ok()
    }
}

struct ParseState<'ast> {
    arena: &'ast Arena,
    cur_attachment: Attachment<'ast>,
}

impl<'ast> ParseState<'ast> {
    fn new(arena: &'ast Arena) -> ParseState<'ast> {
        ParseState {
            arena,
            cur_attachment: Default::default()
        }
    }
}