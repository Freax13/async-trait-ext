mod method;
mod r#trait;

use r#trait::TraitData;

use heck::CamelCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use syn::{parse_macro_input, Error, ItemTrait, TraitItemMethod};

#[proc_macro_attribute]
pub fn async_trait_ext(attr: TokenStream, item: TokenStream) -> TokenStream {
    let meta = format!("{}", attr);
    let dynamic = match meta.as_str() {
        "" => false,
        "dynamic" => true,
        _ => {
            let stream: proc_macro2::TokenStream = attr.into();
            return Error::new_spanned(stream, format!("expected dynamic got {}", meta))
                .to_compile_error()
                .into();
        }
    };

    let item = parse_macro_input!(item as ItemTrait);
    match TraitData::new(&item, dynamic) {
        Ok(data) => data.to_tokens(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn method_struct_name(item: &ItemTrait, m: &TraitItemMethod) -> Ident {
    let raw = format!("{}_{}", &item.ident, &m.sig.ident);
    let camel_case = raw.to_camel_case();
    Ident::new(&camel_case, Span::call_site())
}
