use crate::ast;
use crate::{Ast, Parse, ParseError, Parser, Peek, Spanned};
use runestick::Span;

/// A function.
#[derive(Debug, Clone, Ast, Spanned)]
pub struct ItemFn {
    /// The attributes for the fn
    #[spanned(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The visibility of the `fn` item
    #[spanned(iter)]
    pub visibility: Option<ast::Visibility>,
    /// The optional `async` keyword.
    #[spanned(iter)]
    pub async_: Option<ast::Async>,
    /// The `fn` token.
    pub fn_: ast::Fn,
    /// The name of the function.
    pub name: ast::Ident,
    /// The arguments of the function.
    // TODO: merge args and output into a signature
    pub args: ast::Parenthesized<ast::FnArg, ast::Comma>,
    /// The return type-hint
    #[spanned(iter)]
    pub output: Option<ast::ReturnType>,
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
        matches!(
            self.args.items.first().map(|arg| (&arg.0.ident, &arg.1)),
            Some((ast::FnArgIdent::Self_(..), _))
        )
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
            output: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

impl Peek for ItemFn {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Fn | ast::Kind::Async)
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
/// let item = parse_all::<ast::ItemFn>(r#"
///     #[inline]
///     pub async fn get(url: String) -> String  {
///         http::get(url).await?
///     }"#).unwrap();
/// assert_eq!(item.args.items.len(), 1);
/// let type_ = item.args.items.first().and_then(|(arg, _)| arg.type_.as_ref());
/// assert!(type_.is_some());
/// ```
impl Parse for ItemFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
