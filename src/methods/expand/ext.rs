use crate::{
    ext_trait_name,
    input::{MethodInput, TraitInput},
    methods::{expand::GenericsExpand, future_type, CleanUpMutPatternsExpand},
};
use macro_compose::{Collector, Context, Expand};
use quote::format_ident;
use syn::{
    parse_quote,
    visit_mut::{
        visit_angle_bracketed_generic_arguments_mut, visit_block_mut, visit_ident_mut,
        visit_path_mut, VisitMut,
    },
    AngleBracketedGenericArguments, Expr, FnArg, Ident, ItemTrait, Pat, Path, TraitItemMethod,
    TypeGenerics,
};

pub struct ExtMethodExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for ExtMethodExpand<'_> {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        let mut item = input.clone();
        item.sig.asyncness.take()?;
        item.default.take();

        let generics_expand = GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('_),
        };
        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&generics_expand)?;

        let (_, ty_generics, _) = generics.split_for_impl();

        let mut ctx = Context::new_by_ref(c, &ty_generics);
        let args = ctx.capture(&RenameSelfExpand);

        let future_type = future_type(self.0, input);
        item.sig.output = parse_quote!(-> #future_type #args);

        MethodInput::strip(&mut item.attrs);

        Some(item)
    }
}

struct RenameSelfExpand;

impl Expand<TypeGenerics<'_>> for RenameSelfExpand {
    type Output = AngleBracketedGenericArguments;

    fn expand(&self, input: &TypeGenerics<'_>, _: &mut Collector) -> Option<Self::Output> {
        let mut args = parse_quote!(#input);
        visit_angle_bracketed_generic_arguments_mut(&mut RenameSelfVisit, &mut args);
        Some(args)
    }
}

struct RenameSelfVisit;

impl VisitMut for RenameSelfVisit {
    fn visit_path_mut(&mut self, i: &mut Path) {
        if i.is_ident("__Self") {
            *i = parse_quote!(Self);
        }

        visit_path_mut(self, i);
    }
}

pub struct MethodExtImplExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for MethodExtImplExpand<'_> {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        input.sig.asyncness.as_ref()?;
        if input.default.is_some() {
            return None;
        }

        let mut ctx = Context::new_by_ref(c, input);
        let mut item = ctx.capture(&ExtMethodExpand(self.0))?;

        let args = item
            .sig
            .inputs
            .iter_mut()
            .enumerate()
            .map(|(i, arg)| -> syn::Expr {
                match arg {
                    FnArg::Receiver(_) => parse_quote!(self),
                    FnArg::Typed(pt) => match &*pt.pat {
                        Pat::Ident(pi) => {
                            let ident = &pi.ident;
                            parse_quote!(#ident)
                        }
                        _ => {
                            let ident = format_ident!("arg{}", i);
                            pt.pat = parse_quote!(#ident);
                            parse_quote!(#ident)
                        }
                    },
                }
            });

        let generics_expand = GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('__default_lifetime),
        };
        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&generics_expand)?;

        let args = args.chain(
            generics
                .type_params()
                .map(|_| parse_quote!(::core::marker::PhantomData)),
        );
        let args = args.chain(
            generics
                .lifetimes()
                .map(|_| parse_quote!(::core::marker::PhantomData)),
        );

        let future_type = future_type(self.0, input);
        item.default = Some(parse_quote!(
            {
                #future_type ( #(#args),* )
            }
        ));

        Some(item)
    }
}

pub struct StaticProvidedMethodImplExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for StaticProvidedMethodImplExpand<'_> {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        let attrs = TraitInput::from(self.0.attrs.as_slice());
        if attrs.dynamic.is_some() {
            return None;
        }
        input.sig.asyncness.as_ref()?;

        let default = input.default.clone().take()?;

        let mut ctx = Context::new_by_ref(c, input);
        let mut item = ctx.capture(&ExtMethodExpand(self.0))?;

        item.default = Some(parse_quote!(
            {
                async move {
                    #default
                }
            }
        ));

        Some(item)
    }
}

pub struct DynamicProvidedMethodImplExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for DynamicProvidedMethodImplExpand<'_> {
    type Output = TraitItemMethod;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        let attrs = TraitInput::from(self.0.attrs.as_slice());
        attrs.dynamic?;
        input.sig.asyncness.as_ref()?;

        let mut default = input.default.clone().take()?;
        visit_block_mut(&mut RenameselfExpand, &mut default);

        let mut ctx = Context::new_by_ref(c, input);
        let mut item = ctx.capture(&ExtMethodExpand(self.0))?;

        let ext_ident = ext_trait_name(self.0);
        let params = input.sig.inputs.iter().map(|arg| {
            if let FnArg::Receiver(r) = arg {
                let mutability = &r.mutability;
                parse_quote!(this: & #mutability dyn #ext_ident)
            } else {
                arg.clone()
            }
        });

        let args = item
            .sig
            .inputs
            .iter_mut()
            .enumerate()
            .map(|(i, arg)| -> Expr {
                match arg {
                    FnArg::Receiver(_) => parse_quote!(self),
                    FnArg::Typed(pt) => match &mut *pt.pat {
                        Pat::Ident(i) => {
                            let ident = &mut i.ident;
                            if ident == "self" {
                                *ident = format_ident!("this", span = ident.span());
                            }
                            parse_quote!(#ident)
                        }
                        _ => {
                            let ident = format_ident!("arg{}", i);
                            pt.pat = parse_quote!(#ident);
                            parse_quote!(#ident)
                        }
                    },
                }
            });

        let generics_expand = GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('__default_lifetime),
        };
        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&generics_expand)?;
        let (_, ty_generics, where_clause) = generics.split_for_impl();

        let output = &input.sig.output;
        item.default = Some(parse_quote!(
            {
                async fn fn_impl #ty_generics( #(#params),* ) #output #where_clause {
                    #default
                }
                fn_impl( #(#args),* )
            }
        ));

        let mut ctx = Context::new(c, item);
        ctx.capture(&CleanUpMutPatternsExpand)
    }
}

struct RenameselfExpand;

impl VisitMut for RenameselfExpand {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        if i == "self" {
            *i = format_ident!("this", span = i.span());
        }
        visit_ident_mut(self, i);
    }
}
