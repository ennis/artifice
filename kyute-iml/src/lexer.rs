use logos::Lexer;
use logos::Logos;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LexicalError {
    #[error("invalid int literal: {0}")]
    InvalidIntLiteral(#[from] std::num::ParseIntError),
    #[error("invalid float literal: {0}")]
    InvalidFloatLiteral(#[from] std::num::ParseFloatError),
}

fn parse_str<'input>(lex: &mut logos::Lexer<'input, Token<'input>>) -> &'input str {
    let s = lex.slice();
    &s[1..s.len() - 1]
}

fn parse_f32<'input>(lex: &mut Lexer<'input, Token<'input>>) -> Result<f32, LexicalError> {
    let s = lex.slice();
    Ok(f32::from_str(
        s.strip_suffix(|c| c == 'f' || c == 'F').unwrap_or(s),
    )?)
}

fn parse_f64<'input>(lex: &mut Lexer<'input, Token<'input>>) -> Result<f64, LexicalError> {
    let s = lex.slice();
    Ok(f64::from_str(
        s.strip_suffix(|c| c == 'f' || c == 'F')
            .and_then(|s| s.strip_suffix(|c| c == 'l' || c == 'L'))
            .unwrap_or(s),
    )?)
}

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'input> {
    #[regex(r#""([^\\"]*)""#, parse_str)]
    Str(&'input str),
    #[regex(r#"[a-zA-Z_][a-zA-Z0-9_]*"#)]
    Ident,
    #[token("true", |_| true)]
    #[token("false", |_| false)]
    BoolConst(bool),
    #[regex(
        r"([0-9]+\.[0-9]+|[0-9]+\.|\.[0-9]+)([eE][+-]?[0-9]+)?(f|F)?",
        parse_f32
    )]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+(f|F)?", parse_f32)]
    FloatConst(f32),
    #[regex(
        r"([0-9]+\.[0-9]+|[0-9]+\.|\.[0-9]+)([eE][+-]?[0-9]+)?(lf|LF)",
        parse_f64
    )]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+(lf|LF)", parse_f64)]
    DoubleConst(f64),
    #[regex("//.*", logos::skip)]
    SingleLineComment,
    #[regex(r"/\*([^*]|\*[^/])+\*/", logos::skip)]
    BlockComment,
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
    #[token("void")]
    Void,
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
    #[regex("[ \r\n]")]
    Newline,
    #[regex("[ \t\r\n]", logos::skip)]
    Whitespace,
    #[error]
    Error,
}
