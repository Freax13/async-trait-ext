use crate::{
    ext_trait_name,
    input::TraitInput,
    methods::{
        CleanUpMutPatternsExpand, ExtMethodExpand, FutureAliasExpand, FutureStructExpand,
        ImplFutureExpand, PollMethodExpand,
    },
};
use macro_compose::{Collector, Context, Expand};
use syn::{parse_quote, ItemTrait, TraitItem};

pub struct PollTraitExpand;

impl Expand<ItemTrait> for PollTraitExpand {
    type Output = ItemTrait;

    fn expand(&self, input: &ItemTrait, c: &mut Collector) -> Option<Self::Output> {
        let mut item = input.clone();

        item.items = item
            .items
            .into_iter()
            .filter_map(|item| {
                if let TraitItem::Method(m) = item {
                    let mut subcontext = Context::new(c, m);
                    let method = subcontext.capture(&PollMethodExpand(input))?;

                    let mut subcontext = Context::new(c, method);

                    Some(TraitItem::Method(
                        subcontext.capture(&CleanUpMutPatternsExpand)?,
                    ))
                } else {
                    Some(item)
                }
            })
            .collect();

        for item in item.items.iter_mut() {
            if let TraitItem::Method(m) = item {
                let mut subcontext = Context::new_by_ref(c, m);

                if let Some(res) = subcontext.capture(&PollMethodExpand(input)) {
                    *m = res;
                }
            }
        }

        TraitInput::strip(&mut item.attrs);

        Some(item)
    }
}

pub struct ExtensionTraitExpand;

impl Expand<ItemTrait> for ExtensionTraitExpand {
    type Output = ItemTrait;

    fn expand(&self, input: &ItemTrait, c: &mut Collector) -> Option<Self::Output> {
        let mut item = input.clone();
        item.ident = ext_trait_name(input);

        let ident = &input.ident;
        item.supertraits.push(parse_quote!(#ident));

        let attrs = TraitInput::from(item.attrs.as_slice());
        if attrs.dynamic.is_none() {
            item.supertraits.push(parse_quote!(::core::marker::Sized));
        }

        item.items = item
            .items
            .into_iter()
            .filter_map(|item| {
                if let TraitItem::Method(m) = item {
                    let mut subcontext = Context::new(c, m);

                    subcontext.expand(&FutureStructExpand(input));
                    subcontext.expand(&ImplFutureExpand(input));
                    subcontext.expand(&FutureAliasExpand(input));

                    let method = subcontext.capture(&ExtMethodExpand(input))?;
                    let mut subcontext = Context::new(c, method);
                    subcontext
                        .capture(&CleanUpMutPatternsExpand)
                        .map(TraitItem::Method)
                } else {
                    None
                }
            })
            .collect();

        TraitInput::strip(&mut item.attrs);

        Some(item)
    }
}
