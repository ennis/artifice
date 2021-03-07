use logos::Lexer;
use logos::Logos;
use regex::internal::Input;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use crate::Arena;
use std::panic::panic_any;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LexicalError {
    #[error("invalid int literal: {0}")]
    InvalidIntLiteral(#[from] std::num::ParseIntError),
    #[error("invalid float literal: {0}")]
    InvalidFloatLiteral(#[from] std::num::ParseFloatError),
}

fn parse_int<'i>(lex: &mut logos::Lexer<'i, Token<'i>>, radix: u32) -> Result<i128, LexicalError> {
    Ok(i128::from_str_radix(lex.slice(), radix)
        .map_err(|err| LexicalError::InvalidIntLiteral(err))?)
}

fn parse_float<'i>(lex: &mut logos::Lexer<'i, Token<'i>>) -> Result<f64, LexicalError> {
    Ok(f64::from_str(lex.slice()).map_err(|err| LexicalError::InvalidFloatLiteral(err))?)
}

fn parse_str<'input>(lex: &mut logos::Lexer<'input, Token<'input>>) -> &'input str {
    let s = lex.slice();
    &s[1..s.len() - 1]
}

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token<'input> {
    //------------------- Identifiers -------------------
    #[regex(r#"[a-zA-Z_][a-zA-Z0-9_]*"#)]
    Ident(&'input str),

    //------------------- Keywords -------------------
    #[token("pub")]
    Pub,
    #[token("const")]
    Const,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("RenderPass")]
    RenderPass,
    #[token("Attachment")]
    Attachment,
    #[token("Subpass")]
    Subpass,

    //------------------- Primitive types -------------------
    #[token("u32x2")] U32x2,
    #[token("u32x3")] U32x3,
    #[token("u32x4")] U32x4,
    #[token("i32x2")] I32x2,
    #[token("i32x3")] I32x3,
    #[token("i32x4")] I32x4,
    #[token("f32x2")] F32x2,
    #[token("f32x3")] F32x3,
    #[token("f32x4")] F32x4,

    //------------------- Contextual keywords -------------------
    /*#[token("flags")] CkFlags,
    #[token("format")] CkFormat,
    #[token("samples")] CkSamples,
    #[token("load_op")] CkLoadOp,
    #[token("store_op")] CkStoreOp,
    #[token("stencil_load_op")] CkStencilLoadOp,
    #[token("stencil_store_op")] CkStencilStoreOp,
    #[token("initial_layout")] CkInitialLayout,
    #[token("final_layout")] CkFinalLayout,

    #[token("MAY_ALIAS")] CkRenderPassFlagsMayAlias,
    #[token("DONT_CARE")] CkDontCare,
    #[token("STORE")] CkStoreOpStore,
    #[token("LOAD")] CkLoadOpLoad,*/
    //------------------- Contextual keywords: image layouts -------------------

    //------------------- Literals -------------------
    #[token("true", |_| true)]
    #[token("false", |_| false)]
    BoolLiteral(bool),
    #[regex(r"0b[0-1_]*[0-1][0-1_]*", |lex| parse_int(lex, 2))]
    #[regex(r"0o[0-7_]*[0-7][0-7_]*", |lex| parse_int(lex, 8))]
    #[regex(r"[0-9][0-9_]*", |lex| parse_int(lex, 10))]
    #[regex(r"0[xX][0-9A-Fa-f_]*[0-9A-Fa-f][0-9A-Fa-f_]*", |lex| parse_int(lex, 16))]
    IntLiteral(i128),

    #[regex(r#""([^\\"]*)""#, parse_str)]
    String(&'input str),

    #[regex("[0-9][0-9_]*[.]", parse_float)]
    #[regex("[0-9][0-9_]*(?:[eE][+-]?[0-9_]*[0-9][0-9_]*)", parse_float)]
    #[regex(
        "[0-9][0-9_]*[.][0-9][0-9_]*(?:[eE][+-]?[0-9_]*[0-9][0-9_]*)?",
        parse_float
    )]
    //#[regex("[+-]?[0-9][0-9_]*([.][0-9][0-9_]*)?(?:[eE][+-]?[0-9_]*[0-9][0-9_]*)?(f32|f64)")]
    FloatLiteral(f64),

    //------------------- Comments -------------------
    #[regex("//.*", logos::skip)]
    SingleLineComment,
    #[regex(r"/\*([^*]|\*[^/])+\*/", logos::skip)]
    BlockComment,
    //------------------- Operators -------------------
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(".")]
    Dot,
    #[token("++")]
    Inc,
    #[token("--")]
    Dec,
    #[token("+")]
    Plus,
    #[token("-")]
    Dash,
    #[token("!")]
    Bang,
    #[token("~")]
    Tilde,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("<<")]
    LShift,
    #[token(">>")]
    RShift,
    #[token("<")]
    LAngle,
    #[token(">")]
    RAngle,
    #[token("<=")]
    LEqual,
    #[token(">=")]
    REqual,
    #[token("==")]
    DEqual,
    #[token("!=")]
    BangEqual,
    #[token("&")]
    Ampersand,
    #[token("^")]
    Caret,
    #[token("|")]
    Bar,
    #[token("&&")]
    And,
    #[token("^^")]
    Xor,
    #[token("||")]
    Or,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("=")]
    Equal,
    #[token("*=")]
    StarEqual,
    #[token("/=")]
    SlashEqual,
    #[token("%=")]
    PercentEqual,
    #[token("+=")]
    PlusEqual,
    #[token("-=")]
    DashEqual,
    #[token("<<=")]
    LShiftEqual,
    #[token(">>=")]
    RShiftEqual,
    #[token("&=")]
    AmpersandEqual,
    #[token("^=")]
    CaretEqual,
    #[token("|=")]
    BarEqual,
    #[token(",")]
    Comma,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    //------------------- Whitespace -------------------
    #[regex("[ \t\r\n]", logos::skip)]
    Whitespace,
    //------------------- Other -------------------
    #[error]
    Error,
}


impl<'input> Token<'input> {
     pub(crate) fn copy_str<'ast>(&self, arena: &'ast Arena) -> &'ast str {
         match self {
             Token::String(s) => arena.alloc_str(s),
             Token::Ident(s) => arena.alloc_str(s),
             _ => panic!("cannot convert {:?} into a string", self)
         }
     }

    /// Extract a string from the token if it represents a string
    pub(crate) fn as_str(&self) -> &'input str {
        match self {
            Token::String(s) => s,
            Token::Ident(s) => s,
            _ => panic!("cannot convert {:?} into a string", self)
        }
    }

    /// Extract an f64 value from the token if it represents a floating-point literal
    pub(crate) fn as_f64(&self) -> f64 {
        match self {
            Token::FloatLiteral(f) => *f,
            _ => panic!("cannot convert {:?} into f64", self)
        }
    }

    /// Extract an i64 value from the token if it represents an integer literal
    pub(crate) fn as_i64(&self) -> i64 {
        match self {
            Token::IntLiteral(f) => *f as i64,
            _ => panic!("cannot convert {:?} into i64", self)
        }
    }

    /// Extract a bool value from the token if it represents a boolean literal
    pub(crate) fn as_bool(&self) -> bool {
        match self {
            Token::BoolLiteral(b) => *b,
            _ => panic!("cannot convert {:?} into bool", self)
        }
    }
}


/// Bridge between a logos lexer and what LALRPOP expects
pub struct LexerAdapter<'input> {
    lexer: logos::Lexer<'input, Token<'input>>,
}

impl<'input> LexerAdapter<'input> {
    pub fn new(input: &'input str) -> LexerAdapter {
        LexerAdapter {
            lexer: Token::lexer(input),
        }
    }
}

impl<'input> Iterator for LexerAdapter<'input> {
    type Item = (usize, Token<'input>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|token| {
            let span = self.lexer.span();
            (span.start, token, span.end)
        })
    }
}
