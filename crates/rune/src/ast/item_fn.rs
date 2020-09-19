use crate::ast;
use crate::{IntoTokens, Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// A function.
#[derive(Debug, Clone)]
pub struct ItemFn {
    /// The attributes for the fn
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `fn` item
    pub visibility: Option<ast::Visibility>,
    /// The optional `async` keyword.
    pub async_: Option<ast::Async>,
    /// The `fn` token.
    pub fn_: ast::Fn,
    /// The name of the function.
    pub name: ast::Ident,
    /// The arguments of the function.
    pub args: ast::Parenthesized<ast::FnArg, ast::Comma>,
    /// The body of the function.
    pub body: ast::Block,
}

impl ItemFn {
    /// Get the identifying span for this function.
    pub fn item_span(&self) -> Span {
        if let Some(async_) = &self.async_ {
            async_.span().join(self.args.span())
        } else {
            self.fn_.span().join(self.args.span())
        }
    }

    /// Test if function is an instance fn.
    pub fn is_instance(&self) -> bool {
        matches!(self.args.items.first(), Some((ast::FnArg::Self_(..), _)))
    }

    /// Parse a `fn` item with the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser<'_>,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        Ok(Self {
            attributes,
            visibility: parser.parse()?,
            async_: parser.parse()?,
            fn_: parser.parse()?,
            name: parser.parse()?,
            args: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

impl Spanned for ItemFn {
    fn span(&self) -> Span {
        let start = if let Some(first) = self.attributes.first() {
            first.span()
        } else if let Some(visibility) = &self.visibility {
            visibility.span()
        } else if let Some(async_) = &self.async_ {
            async_.span()
        } else {
            self.fn_.span()
        };

        start.join(self.body.span())
    }
}

impl Peek for ItemFn {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        let t = match t1 {
            Some(t) => t,
            None => return false,
        };

        matches!(t.kind, ast::Kind::Fn | ast::Kind::Async)
    }
}

/// Parse implementation for a function.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ItemFn>("async fn hello() {}").unwrap();
/// assert!(parse_all::<ast::ItemFn>("fn async hello() {}").is_err());
///
/// let item = parse_all::<ast::ItemFn>("fn hello() {}").unwrap();
/// assert_eq!(item.args.items.len(), 0);
///
/// let item = parse_all::<ast::ItemFn>("fn hello(foo, bar) {}").unwrap();
/// assert_eq!(item.args.items.len(), 2);
///
/// let item = parse_all::<ast::ItemFn>("pub fn hello(foo, bar) {}").unwrap();
/// let item = parse_all::<ast::ItemFn>("pub async fn hello(foo, bar) {}").unwrap();
/// let item = parse_all::<ast::ItemFn>("#[inline] fn hello(foo, bar) {}").unwrap();
/// let item = parse_all::<ast::ItemFn>("#[inline] pub async fn hello(foo, bar) {}").unwrap();
///
/// if let Some(ast::Visibility::Public(_)) = &item.visibility {} else {
///     panic!("expected `fn` item visibility of `Public` got {:?}", &item.visibility);
/// }
/// assert_eq!(item.args.items.len(), 2);
/// assert_eq!(item.attributes.len(), 1);
///
/// ```
impl Parse for ItemFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}

impl IntoTokens for ItemFn {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.async_.into_tokens(context, stream);
        self.visibility.into_tokens(context, stream);
        self.fn_.into_tokens(context, stream);
        self.name.into_tokens(context, stream);
        self.args.into_tokens(context, stream);
        self.body.into_tokens(context, stream);
    }
}
