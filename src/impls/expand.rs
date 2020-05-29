use crate::{
    ext_trait_name,
    methods::{
        DynamicProvidedMethodImplExpand, MethodExtImplExpand, StaticProvidedMethodImplExpand,
    },
};
use macro_compose::{Collector, Context, Expand};
use syn::{parse_quote, ItemImpl, ItemTrait, TraitItem};

pub struct ImplExtTraitExpand;

impl Expand<ItemTrait> for ImplExtTraitExpand {
    type Output = ItemImpl;

    fn expand(&self, input: &ItemTrait, c: &mut Collector) -> Option<Self::Output> {
        let methods = input
            .items
            .iter()
            .filter_map(|i| {
                if let TraitItem::Method(m) = i {
                    Some(m.clone())
                } else {
                    None
                }
            })
            .flat_map(|m| {
                let mut subcontext = Context::new(c, m);
                subcontext
                    .capture(&MethodExtImplExpand(input))
                    .into_iter()
                    .chain(subcontext.capture(&StaticProvidedMethodImplExpand(input)))
                    .chain(subcontext.capture(&DynamicProvidedMethodImplExpand(input)))
            });

        let trait_ident = &input.ident;
        let ext_ident = ext_trait_name(input);

        Some(parse_quote!(
            impl<__IMPL: #trait_ident> #ext_ident for __IMPL {
                #(#methods)*
            }
        ))
    }
}
