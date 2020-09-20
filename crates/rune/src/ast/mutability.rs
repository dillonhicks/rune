use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, ToTokens};

/// Represents either `mut` or `const` in type contexts.
///
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum Mutability {
    /// The `const` keyword
    Const(ast::Const),
    /// The `mut` keyword
    Mut(ast::Mut),
}

impl Mutability {
    /// Return `true` if `mut`
    pub const fn is_mut(&self) -> bool {
        matches!(self, Mutability::Mut(_))
    }

    /// Return `true` if `const`
    pub const fn is_const(&self) -> bool {
        matches!(self, Mutability::Const(_))
    }
}

impl Peek for Mutability {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Const | ast::Kind::Mut)
    }
}

impl Parse for Mutability {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;
        match token.kind {
            ast::Kind::Const => Ok(Mutability::Const(parser.parse()?)),
            ast::Kind::Mut => Ok(Mutability::Mut(parser.parse()?)),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedMutability { actual: token.kind },
            )),
        }
    }
}
