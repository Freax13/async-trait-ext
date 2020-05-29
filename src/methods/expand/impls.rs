use crate::{
    input::TraitInput,
    methods::{expand::GenericsExpand, future_type},
};
use macro_compose::{Collector, Context, Expand};
use proc_macro2::Span;
use quote::format_ident;
use std::iter::FromIterator;
use syn::{
    parse_quote, punctuated::Punctuated, Expr, GenericParam, Index, ItemImpl, ItemTrait, Member,
    ReturnType, TraitItemMethod,
};

pub struct ImplFutureExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for ImplFutureExpand<'_> {
    type Output = ItemImpl;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        input.sig.asyncness?;
        if input.default.is_some() {
            return None;
        }

        let attrs = TraitInput::from(self.0.attrs.as_slice());

        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('__default_lifetime),
        })?;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let output = match &input.sig.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty.clone(),
        };

        let args = input.sig.inputs.iter().enumerate().map(|(idx, _)| {
            Member::Unnamed(Index {
                index: idx as u32,
                span: Span::call_site(),
            })
        });

        let (_, trait_ty_generics, _) = self.0.generics.split_for_impl();
        let trait_turbofish = trait_ty_generics.as_turbofish();

        let mut generics = input.sig.generics.clone();
        generics.params = Punctuated::from_iter(
            input
                .sig
                .generics
                .params
                .iter()
                .filter(|p| !matches!(p, GenericParam::Lifetime(_)))
                .cloned(),
        );
        if generics.params.is_empty() {
            generics.lt_token = None;
            generics.gt_token = None;
        }
        let (_, method_ty_generics, _) = generics.split_for_impl();
        let method_turbofish = method_ty_generics.as_turbofish();

        let trait_ident = &self.0.ident;
        let method_ident = format_ident!("poll_{}", input.sig.ident);

        let path: Expr = if attrs.dynamic.is_some() {
            parse_quote!(#trait_ident #trait_turbofish :: #method_ident #method_turbofish)
        } else {
            parse_quote!(<__Self as #trait_ident #trait_turbofish >:: #method_ident #method_turbofish)
        };

        let future_type = future_type(self.0, input);
        Some(parse_quote!(
            impl #impl_generics ::core::future::Future for #future_type #ty_generics #where_clause {
                type Output = #output;

                fn poll(mut self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context) -> ::core::task::Poll<Self::Output> {
                    let this = &mut *self;
                    #path ( #(this. #args .into(),)* cx)
                }
            }
        ))
    }
}
