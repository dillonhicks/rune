use crate::ast;
use crate::{Ast, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned};

/// The type of a funtion argument
#[derive(Debug, Clone, Ast, Spanned, Parse)]
pub struct ReturnType {
    /// The `->` preceeding the type
    pub arrow: ast::RArrow,
    /// The type that is returned
    pub type_: Box<ast::Type>,
}

impl Peek for ReturnType {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::RArrow) && ast::Type::peek(t2, None)
    }
}
