use crate::{ast, ParseError, Parser};
use crate::{Parse, Peek, Spanned, ToTokens};

/// Represents a type
#[derive(Debug, Clone, ToTokens, Spanned)]
pub enum Type {
    /// The `_` type to be inferred by the compiler
    Infer(ast::TypeInfer),
    /// The `!` type
    Never(ast::TypeNever),
    /// A path to a type
    Path(ast::TypePath),
    /// A pointer type: `*const T`
    Pointer(ast::TypePtr),
    /// The `...` type in `extern` functions
    Variadic(ast::TypeVariadic),
}

impl Parse for Type {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Type::Path(parser.parse()?))
    }
}

impl Peek for Type {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        ast::TypePath::peek(t1, t2)
            || ast::TypePtr::peek(t1, t2)
            || ast::TypeNever::peek(t1, t2)
            || ast::TypeInfer::peek(t1, t2)
            || ast::TypeVariadic::peek(t1, t2)
    }
}
