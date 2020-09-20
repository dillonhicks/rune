use crate::{ast, ParseError, Parser};
use crate::{Ast, Parse, Peek, Spanned, TokenStream};

/// Represents a type
#[derive(Debug, Clone, Ast, Spanned)]
pub enum Type {
    /// A path to a type
    Path(ast::TypePath),
}

impl Parse for Type {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Type::Path(parser.parse()?))
    }
}

impl Peek for Type {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::TypePath::peek(t1, t2)
    }
}
