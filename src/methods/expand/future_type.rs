use crate::{
    input::TraitInput,
    methods::{
        expand::{GenericsExpand, NeedsDefaultLifetime},
        future_type, RenameSelf,
    },
};
use macro_compose::{Collector, Context, Expand};
use syn::{
    parse_quote,
    visit_mut::{visit_fn_arg_mut, visit_type_mut},
    FnArg, ItemStruct, ItemTrait, ItemType, Lifetime, ReturnType, TraitItemMethod,
};

pub struct FutureStructExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for FutureStructExpand<'_> {
    type Output = ItemStruct;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        input.sig.asyncness?;
        if input.default.is_some() {
            return None;
        }

        let future_type = future_type(self.0, input);
        let default_lifetime: Lifetime = parse_quote!('__default_lifetime);

        let attrs = TraitInput::from(self.0.attrs.as_slice());

        let mut args = input.sig.inputs.clone();
        let mut ndl = NeedsDefaultLifetime { res: false };
        for arg in args.iter_mut() {
            visit_fn_arg_mut(&mut ndl, arg);
        }

        let args = args.iter().map(|arg| match arg {
            FnArg::Receiver(r) => {
                let reference = r.reference.as_ref();
                let lifetime = reference.map(|r| {
                    r.1.as_ref()
                        .cloned()
                        .unwrap_or_else(|| default_lifetime.clone())
                });
                let reference = reference.map(|r| r.0);

                let mutability = r.mutability;
                if attrs.dynamic.is_some() {
                    let (_, ty_generics, _) = self.0.generics.split_for_impl();
                    let ident = &self.0.ident;
                    parse_quote!(#reference #lifetime #mutability dyn #ident #ty_generics)
                } else {
                    parse_quote!(#reference #lifetime #mutability __Self)
                }
            }
            FnArg::Typed(pt) => *pt.ty.clone(),
        });
        let args = args.map(|mut arg| {
            visit_type_mut(&mut RenameSelf, &mut arg);
            arg
        });

        let generics_expand = GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('__default_lifetime),
        };
        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&generics_expand)?;
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let args = args.chain(generics.type_params().map(|tp| {
            let ident = &tp.ident;
            parse_quote!(::core::marker::PhantomData<*const #ident>)
        }));
        let args = args.chain(generics.lifetimes().map(|tp| {
            let lifetime = &tp.lifetime;
            parse_quote!(::core::marker::PhantomData<& #lifetime ()>)
        }));

        let comment = format!(
            " the future returned by [`{}Ext::{}`]",
            self.0.ident, input.sig.ident
        );

        let vis = &self.0.vis;
        Some(parse_quote!(
            #[doc = #comment ]
            #vis struct #future_type #impl_generics (#(#args),*) #where_clause;
        ))
    }
}

pub struct FutureAliasExpand<'a>(pub &'a ItemTrait);

impl Expand<TraitItemMethod> for FutureAliasExpand<'_> {
    type Output = ItemType;

    fn expand(&self, input: &TraitItemMethod, c: &mut Collector) -> Option<Self::Output> {
        input.sig.asyncness?;
        input.default.as_ref()?;

        let ty = match &input.sig.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty.clone(),
        };

        let generics_expand = GenericsExpand {
            item: self.0,
            default_lifetime: parse_quote!('__default_lifetime),
        };
        let mut ctx = Context::new_by_ref(c, input);
        let generics = ctx.capture(&generics_expand)?;
        let (impl_generics, _, where_clause) = generics.split_for_impl();

        let comment = format!(
            " the future returned by [`{}Ext::{}`]",
            self.0.ident, input.sig.ident
        );
        let vis = &self.0.vis;
        let future_type = future_type(self.0, input);
        Some(parse_quote!(
            #[doc = #comment ]
            #vis type #future_type #impl_generics = impl ::core::future::Future<Output = #ty > #where_clause;
        ))
    }
}
