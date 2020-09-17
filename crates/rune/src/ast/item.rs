use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Peek};

impl_enum_ast! {
    /// A declaration.
    pub enum Item {
        /// An attribute item
        ItemAttribute(ast::Attribute),
        /// A use declaration.
        ItemUse(ast::ItemUse),
        /// A function declaration.
        // large variant, so boxed
        ItemFn(Box<ast::ItemFn>),
        /// An enum declaration.
        ItemEnum(ast::ItemEnum),
        /// A struct declaration.
        ItemStruct(ast::ItemStruct),
        /// An impl declaration.
        ItemImpl(ast::ItemImpl),
        /// A module declaration.
        ItemMod(ast::ItemMod),
        /// A macro call expanding into an item.
        MacroCall(ast::MacroCall),
    }
}

impl Item {
    /// Indicates if the declaration needs a semi-colon or not.
    pub fn needs_semi_colon(&self) -> bool {
        matches!(self, Self::MacroCall(..))
    }

    /// Test if declaration is suitable inside of a block.
    pub fn peek_as_stmt(parser: &mut Parser<'_>) -> Result<bool, ParseError> {
        let t1 = parser.token_peek_pair()?;

        let (t, t2) = match t1 {
            Some(t1) => t1,
            None => return Ok(false),
        };

        Ok(match t.kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async | ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            _ => ast::Attribute::peek(Some(t), t2),
        })
    }
}

impl Peek for Item {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let t = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t.kind {
            ast::Kind::Use => true,
            ast::Kind::Enum => true,
            ast::Kind::Struct => true,
            ast::Kind::Impl => true,
            ast::Kind::Async | ast::Kind::Fn => true,
            ast::Kind::Mod => true,
            ast::Kind::Ident(..) => true,
            _ => ast::Attribute::peek(t1, t2),
        }
    }
}

impl Parse for Item {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let t = parser.token_peek_eof()?;

        Ok(match t.kind {
            ast::Kind::Use => Self::ItemUse(parser.parse()?),
            ast::Kind::Enum => Self::ItemEnum(parser.parse()?),
            ast::Kind::Struct => Self::ItemStruct(parser.parse()?),
            ast::Kind::Impl => Self::ItemImpl(parser.parse()?),
            ast::Kind::Async | ast::Kind::Fn => Self::ItemFn(Box::new(parser.parse()?)),
            ast::Kind::Mod => Self::ItemMod(parser.parse()?),
            ast::Kind::Ident(..) => Self::MacroCall(parser.parse()?),
            _ => Self::ItemAttribute(parser.parse().map_err(|_err| {
                ParseError::new(t, ParseErrorKind::ExpectedItem { actual: t.kind })
            })?),
        })
    }
}
