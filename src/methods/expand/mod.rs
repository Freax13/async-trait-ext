mod ext;
mod future_type;
mod impls;
mod poll;

pub use ext::{
    DynamicProvidedMethodImplExpand, ExtMethodExpand, MethodExtImplExpand,
    StaticProvidedMethodImplExpand,
};
pub use future_type::{FutureAliasExpand, FutureStructExpand};
pub use impls::ImplFutureExpand;
pub use poll::PollMethodExpand;

use crate::input::TraitInput;
use macro_compose::{Collector, Expand};
use proc_macro2::Span;
use std::iter::FromIterator;
use syn::{
    parse_quote,
    punctuated::Punctuated,
    visit_mut::{
        visit_fn_arg_mut, visit_generics_mut, visit_ident_mut, visit_receiver_mut,
        visit_type_reference_mut, VisitMut,
    },
    FnArg, GenericParam, Generics, Ident, ItemTrait, LifetimeDef, Pat, Receiver, Token,
    TraitItemMethod, TypeReference, WhereClause,
};

struct NeedsDefaultLifetime {
    res: bool,
}

impl VisitMut for NeedsDefaultLifetime {
    fn visit_receiver_mut(&mut self, i: &mut Receiver) {
        if let Some((_, lt)) = i.reference.as_mut() {
            lt.get_or_insert_with(|| {
                self.res = true;
                parse_quote!('__default_lifetime)
            });
        }

        visit_receiver_mut(self, i);
    }

    fn visit_type_reference_mut(&mut self, i: &mut TypeReference) {
        i.lifetime.get_or_insert_with(|| {
            self.res = true;
            parse_quote!('__default_lifetime)
        });

        visit_type_reference_mut(self, i);
    }
}

struct GenericsExpand<'a> {
    item: &'a ItemTrait,
    default_lifetime: LifetimeDef,
}

impl Expand<TraitItemMethod> for GenericsExpand<'_> {
    type Output = Generics;

    fn expand(&self, input: &TraitItemMethod, _: &mut Collector) -> Option<Self::Output> {
        let attrs = TraitInput::from(self.item.attrs.as_slice());

        let mut args = input.sig.inputs.clone();
        let mut ndl = NeedsDefaultLifetime { res: false };
        for arg in args.iter_mut() {
            visit_fn_arg_mut(&mut ndl, arg);
        }

        let default_lifetime_def = if ndl.res {
            Some(self.default_lifetime.clone())
        } else {
            None
        };

        let lifetimes = self
            .item
            .generics
            .lifetimes()
            .chain(input.sig.generics.lifetimes())
            .cloned()
            .chain(default_lifetime_def);

        let self_type: Option<syn::TypeParam> = if attrs.dynamic.is_none() {
            let ident = &self.item.ident;
            let (_, ty_generics, _) = self.item.generics.split_for_impl();
            Some(parse_quote!(__Self: #ident #ty_generics))
        } else {
            None
        };
        let types = self
            .item
            .generics
            .type_params()
            .chain(input.sig.generics.type_params())
            .cloned()
            .chain(self_type);

        let consts = self
            .item
            .generics
            .const_params()
            .chain(input.sig.generics.const_params())
            .cloned();

        let where_clauses = self
            .item
            .generics
            .where_clause
            .as_ref()
            .into_iter()
            .chain(input.sig.generics.where_clause.as_ref())
            .map(|c| c.predicates.iter())
            .flatten()
            .cloned();

        let where_clause = Punctuated::from_iter(where_clauses);
        let where_clause = if where_clause.is_empty() {
            None
        } else {
            Some(WhereClause {
                where_token: Token![where](Span::call_site()),
                predicates: where_clause,
            })
        };

        let params: Punctuated<syn::GenericParam, _> = Punctuated::from_iter(
            lifetimes
                .map(GenericParam::Lifetime)
                .chain(types.map(GenericParam::Type))
                .chain(consts.map(GenericParam::Const)),
        );

        let mut generics = Generics {
            lt_token: Some(Token![<](Span::call_site())),
            params,
            gt_token: Some(Token![>](Span::call_site())),
            where_clause,
        };

        visit_generics_mut(&mut RenameSelf, &mut generics);

        Some(generics)
    }
}

pub struct RenameSelf;

impl VisitMut for RenameSelf {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == "Self" {
            *i = Ident::new("__Self", i.span());
        }
        visit_ident_mut(self, i);
    }
}

pub struct CleanUpMutPatternsExpand;

impl Expand<TraitItemMethod> for CleanUpMutPatternsExpand {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, _: &mut Collector) -> Option<Self::Output> {
        let mut item = input.clone();

        for input in item.sig.inputs.iter_mut() {
            match input {
                FnArg::Receiver(r) => {
                    if r.reference.is_none() {
                        r.mutability.take();
                    }
                }
                FnArg::Typed(pt) => {
                    if let Pat::Ident(ref mut pt) = *pt.pat {
                        pt.mutability.take();
                    }
                }
            }
        }

        Some(item)
    }
}
