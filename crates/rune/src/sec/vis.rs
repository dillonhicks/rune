//! Visibility rules
//!
//! ```rust
//! // file.rs
//!
//! mod a {
//!     pub struct Foo;
//!
//! }
//!
//! mod b {
//!    struct Bar;
//!
//! }
//!
//!
//!
//! ```
use runestick::{Component, Item};

#[derive(Copy, Clone, Debug)]
pub(crate) enum Vis<'a> {
    None,
    Pub,
    Super,
    Crate,
    Path(&'a Item),
    // ...
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum ComponentKind {
    Struct,
    Fn,
    ImplFn,
    StructField,
    EnumVariant,
    Enum,
    Mod,
    File,
}

fn paths_eq(
    isource: &mut dyn Iterator<Item = Component>,
    itarget: &mut dyn Iterator<Item = Component>,
) -> bool {
    loop {
        match isource.next() {
            Some(x) => match itarget.next() {
                Some(y) => {
                    if x != y {
                        return false;
                    }
                }
                None => return false,
            },
            None => return itarget.next().is_none(),
        }
    }
}

fn is_ancestor(
    isource: &mut dyn Iterator<Item = Component>,
    itarget: &mut dyn Iterator<Item = Component>,
) -> bool {
    loop {
        match isource.next() {
            Some(x) => match itarget.next() {
                Some(y) => {
                    if x != y {
                        return false;
                    }
                }
                None => return false,
            },
            None => return itarget.next().is_none(),
        }
    }
}

fn is_parent(
    isource: &mut dyn Iterator<Item = Component>,
    itarget: &mut dyn Iterator<Item = Component>,
) -> bool {
    loop {
        match isource.next() {
            Some(x) => match itarget.next() {
                Some(y) => {
                    if x != y {
                        return false;
                    }
                }
                None => return false,
            },
            None => return itarget.next().is_none(),
        }
    }
}

fn is_same_crate(
    isource: &mut dyn Iterator<Item = Component>,
    itarget: &mut dyn Iterator<Item = Component>,
) -> bool {
    match (isource.next(), itarget.next()) {
        (Some(Component::String(a)), Some(Component::String(b))) => a == b,
        (None, None) => true,
        _ => false,
    }
}

pub(crate) fn is_visible_to(source: &Item, target: &Item, vis: Vis) -> bool {
    match vis {
        Vis::Pub => true,
        Vis::None => paths_eq(&mut source.iter(), &mut target.iter()),
        Vis::Super => false,
        Vis::Crate => is_same_crate(&mut source.iter(), &mut target.iter()),
        Vis::Path(vis_in) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(parts: impl IntoIterator<Item = &'static str>) -> Item {
        let mut item = Item::new();
        for p in parts.into_iter() {
            item.push(p);
        }
        item
    }

    fn not(b: bool) -> bool {
        !b
    }

    #[test]
    fn test_vis_pub() {
        assert!(is_visible_to(&item(vec![]), &item(vec![]), Vis::Pub,));
        assert!(is_visible_to(&item(vec!["a"]), &item(vec!["a"]), Vis::Pub,));
        assert!(is_visible_to(
            &item(vec!["a", "b", "c"]),
            &item(vec!["a", "b", "c"]),
            Vis::Pub,
        ));

        assert!(is_visible_to(
            &item(vec!["a", "b", "c"]),
            &item(vec!["a", "b"]),
            Vis::Pub,
        ));
    }

    #[test]
    fn test_vis_none() {
        assert!(is_visible_to(&item(vec![]), &item(vec![]), Vis::None,));
        assert!(is_visible_to(&item(vec!["a"]), &item(vec!["a"]), Vis::None,));
        assert!(is_visible_to(
            &item(vec!["a", "b", "c"]),
            &item(vec!["a", "b", "c"]),
            Vis::None,
        ));

        assert!(not(is_visible_to(
            &item(vec!["a", "b", "c"]),
            &item(vec!["a", "b"]),
            Vis::None,
        )));
    }
}
