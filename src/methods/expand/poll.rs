use crate::input::MethodInput;
use macro_compose::{Collector, Expand};
use quote::format_ident;
use syn::{self, parse_quote, ItemTrait, ReturnType, TraitItemMethod};

pub struct PollMethodExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for PollMethodExpand<'_> {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, _: &mut Collector) -> Option<Self::Output> {
        let mut item = input.clone();

        let attrs = MethodInput::from(item.attrs.as_slice());

        if item.sig.asyncness.take().is_some() {
            if attrs.provided.is_none() {
                item.sig.ident = format_ident!("poll_{}", input.sig.ident);

                item.sig
                    .inputs
                    .push(parse_quote!(ctx: &mut ::core::task::Context));

                let output = match item.sig.output {
                    ReturnType::Default => parse_quote!(()),
                    ReturnType::Type(_, ty) => *ty,
                };

                item.sig.output = parse_quote!(-> ::core::task::Poll< #output >);
            } else {
                return None;
            }
        }

        MethodInput::strip(&mut item.attrs);

        Some(item)
    }
}
