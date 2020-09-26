use crate::ast;
use crate::ast::expr::{EagerBrace, ExprChain};
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};
use std::fmt;

/// A unary expression.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprUnary {
    /// Token associated with operator.
    pub token: ast::Token,
    /// The expression of the operation.
    pub expr: Box<ast::Expr>,
    /// The operation to apply.
    #[rune(skip)]
    pub op: UnaryOp,
}

/// Parse a unary statement.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprUnary>("!0").unwrap();
/// parse_all::<ast::ExprUnary>("*foo").unwrap();
/// parse_all::<ast::ExprUnary>("&foo").unwrap();
/// ```
impl Parse for ExprUnary {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;
        let op = UnaryOp::from_token(token)?;

        Ok(Self {
            op,
            token,
            expr: Box::new(ast::Expr::parse_primary(
                parser,
                EagerBrace(true),
                ExprChain(true),
                vec![],
            )?),
        })
    }
}

/// A unary operation.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    /// Not `!<thing>`.
    Not,
    /// Reference `&<thing>`.
    BorrowRef,
    /// Dereference `*<thing>`.
    Deref,
}

impl UnaryOp {
    /// Convert a unary operator from a token.
    pub fn from_token(token: ast::Token) -> Result<Self, ParseError> {
        match token.kind {
            ast::Kind::Bang => Ok(Self::Not),
            ast::Kind::Amp => Ok(Self::BorrowRef),
            ast::Kind::Star => Ok(Self::Deref),
            _ => Err(ParseError::expected(token, "unary operator `!`")),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not => write!(fmt, "!")?,
            Self::BorrowRef => write!(fmt, "&")?,
            Self::Deref => write!(fmt, "*")?,
        }

        Ok(())
    }
}
