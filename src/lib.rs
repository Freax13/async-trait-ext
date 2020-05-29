mod impls;
mod input;
mod methods;
mod traits;

use macro_compose::{Collector, Context};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_quote, Ident, ItemTrait};

#[proc_macro_attribute]
pub fn async_trait_ext(input: TokenStream, item: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let item: proc_macro2::TokenStream = item.into();
    let combined: proc_macro2::TokenStream = parse_quote!(#[async_trait_ext( #input )] #item);

    let mut collector = Collector::new();

    let mut trait_context = Context::<ItemTrait>::new_parse2(&mut collector, combined);
    trait_context.lint(&traits::AttributeLint);
    trait_context.lint(&methods::MethodAttrLint);

    trait_context.expand(&traits::PollTraitExpand);
    trait_context.expand(&traits::ExtensionTraitExpand);
    trait_context.expand(&impls::ImplExtTraitExpand);

    collector.finish().into()
}

fn ext_trait_name(input: &ItemTrait) -> Ident {
    format_ident!("{}Ext", input.ident)
}
