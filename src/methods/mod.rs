mod expand;
mod lint;

pub use expand::*;
pub use lint::*;

use heck::CamelCase;
use quote::format_ident;
use syn::{Ident, ItemTrait, TraitItemMethod};

fn future_type(item: &ItemTrait, method: &TraitItemMethod) -> Ident {
    format_ident!(
        "{}{}",
        item.ident,
        method.sig.ident.to_string().to_camel_case(),
        span = method.sig.ident.span()
    )
}
