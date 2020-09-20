use crate::ast;
use crate::{Ast, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned};

/// A single argument in a closure.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::FnArg>("self").unwrap();
/// let arg = parse_all::<ast::FnArg>("x: i32").unwrap();
/// assert!(arg.type_.is_some());
///
/// ```
#[derive(Debug, Clone, Ast, Spanned)]
pub struct FnArg {
    /// Attributes for the function argument
    #[spanned(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// Identifier for the function argument
    pub ident: FnArgIdent,
    /// Optional type of the function argument
    #[spanned(iter)]
    pub type_: Option<FnArgType>,
}

impl FnArg {
    /// Parse a function argument attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(FnArg {
            attributes,
            ident: parser.parse()?,
            type_: parser.parse()?,
        })
    }
}

impl Parse for FnArg {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

/// A single argument in a closure.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::FnArgIdent>("self").unwrap();
/// parse_all::<ast::FnArgIdent>("_").unwrap();
/// parse_all::<ast::FnArgIdent>("abc").unwrap();
/// ```
#[derive(Debug, Clone, Ast, Spanned)]
pub enum FnArgIdent {
    /// The `self` parameter.
    Self_(ast::Self_),
    /// Ignoring the argument with `_`.
    Ignore(ast::Underscore),
    /// Binding the argument to an ident.
    Ident(ast::Ident),
}

impl Parse for FnArgIdent {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Self_ => Self::Self_(parser.parse()?),
            ast::Kind::Underscore => Self::Ignore(parser.parse()?),
            ast::Kind::Ident(..) => Self::Ident(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedFunctionArgument,
                ))
            }
        })
    }
}

/// The type of a function argument
#[derive(Debug, Clone, Ast, Spanned, Parse)]
pub struct FnArgType {
    /// The `:` token between ident and type
    pub colon: ast::Colon,
    /// The type of the argument
    pub type_: Box<ast::Type>,
}

impl Peek for FnArgType {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Colon) && ast::Type::peek(t2, None)
    }
}
