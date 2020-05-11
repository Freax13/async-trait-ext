use crate::method_struct_name;
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::format_ident;
use std::iter::FromIterator;
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote, Block, Error, Expr, ExprPath, Field, Fields, FieldsUnnamed, FnArg, GenericParam,
    Generics, Index, Item, ItemImpl, ItemStruct, ItemTrait, ItemType, Lifetime, Member, Pat,
    PatIdent, PathSegment, Receiver, Result, ReturnType, Token, TraitItemMethod, Type,
    TypeReference, TypeTraitObject, Visibility, WherePredicate,
};

#[derive(Clone)]
pub struct MethodData {
    item: ItemTrait,
    m: TraitItemMethod,
    dynamic: bool,

    need_default_lifetime: bool,
    has_self: bool,

    fields: Vec<Field>,
    poll_args: Vec<Expr>,
}

impl MethodData {
    pub fn new(item: ItemTrait, m: TraitItemMethod, dynamic: bool) -> Result<Self> {
        let mut s = MethodData {
            item,
            m: m.clone(),
            dynamic,
            need_default_lifetime: false,
            has_self: false,
            fields: Vec::new(),
            poll_args: Vec::new(),
        };

        for arg in m.sig.inputs.iter() {
            match arg {
                FnArg::Receiver(r) => s.add_receiver(r)?,
                FnArg::Typed(pat) => s.add_param(*pat.ty.clone(), true),
            };
        }
        s.poll_args.push(parse_quote!(cx));

        // add PhantomData<*const Self> if the trait isn't dynamic and the method doesn't contain self
        if !s.has_self && !dynamic {
            s.add_param(
                parse_quote!(::core::marker::PhantomData<*const Self>),
                false,
            );
            s.has_self = false;
        }

        Ok(s)
    }

    pub fn pollify(&self) -> TraitItemMethod {
        let mut item = self.m.clone();

        // remove async
        item.sig.asyncness.take();

        // add "poll_" to ident
        item.sig.ident = format_ident!("poll_{}", &item.sig.ident);

        // insert cx parameter
        item.sig
            .inputs
            .push(parse_quote!(cx: &mut ::core::task::Context));

        // fix return type
        let return_type = match &item.sig.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty.clone(),
        };
        item.sig.output = parse_quote!(-> ::core::task::Poll<#return_type>);

        item
    }

    pub fn future_struct(&self) -> ItemStruct {
        let comment = format!(
            " the future returned by [`{}Ext::{}`]",
            self.item.ident, self.m.sig.ident
        );

        ItemStruct {
            attrs: vec![parse_quote!(#[doc = #comment])],
            vis: self.item.vis.clone(),
            struct_token: Token![struct](Span::call_site()),
            ident: method_struct_name(&self.item, &self.m),
            generics: self.make_generics(),
            fields: Fields::Unnamed(FieldsUnnamed {
                paren_token: Paren {
                    span: Span::call_site(),
                },
                unnamed: Punctuated::from_iter(self.fields.clone()),
            }),
            semi_token: Some(Token![;](Span::call_site())),
        }
    }

    #[allow(clippy::cmp_owned)]
    pub fn extify(&self, with_content: bool) -> TraitItemMethod {
        let mut item = self.m.clone();

        // remove async
        item.sig.asyncness.take();

        // add Self: Sized
        if !self.dynamic {
            item.sig
                .generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(Self: Sized));
        }

        // remove mut from params
        if !with_content {
            for input in item.sig.inputs.iter_mut() {
                if let FnArg::Typed(pt) = input {
                    if let Pat::Ident(pi) = &mut *pt.pat {
                        if pi.by_ref.is_none() {
                            pi.mutability.take();
                        }
                    }
                }
            }
        }

        // fix return type
        let mut generics = self.make_generics();

        // fix SelfTy -> Self
        generics
            .type_params_mut()
            .filter(|tp| tp.ident.to_string() == "SelfTy")
            .for_each(|tp| tp.ident = Ident::new("Self", Span::call_site()));

        // fix 'default_lifetime -> _
        generics
            .lifetimes_mut()
            .filter(|lt| lt.lifetime.ident.to_string() == "default_lifetime")
            .for_each(|lt| lt.lifetime.ident = Ident::new("_", Span::call_site()));

        let (_, ty_generics, _) = generics.split_for_impl();
        let ident = method_struct_name(&self.item, &self.m);
        item.sig.output = parse_quote!(-> #ident #ty_generics);

        // add default implementation
        if with_content {
            if !self.is_provided() {
                // get args and fix names
                let args = item
                    .sig
                    .inputs
                    .iter_mut()
                    .enumerate()
                    .map(|(i, arg)| -> Expr {
                        match arg {
                            FnArg::Receiver(_) => parse_quote!(self),
                            FnArg::Typed(pat_ty) => match &*pat_ty.pat {
                                Pat::Ident(pat_ident) => {
                                    // the parameter already has a name -> use it
                                    let ident = &pat_ident.ident;
                                    parse_quote!(#ident)
                                }
                                _ => {
                                    // the parameter doesn't have a name -> make one and use it
                                    let ident = format_ident!("arg{}", i + 1);
                                    *pat_ty.pat = Pat::Ident(PatIdent {
                                        attrs: Vec::new(),
                                        by_ref: None,
                                        mutability: None,
                                        ident: ident.clone(),
                                        subpat: None,
                                    });
                                    parse_quote!(#ident)
                                }
                            },
                        }
                    })
                    .chain(if self.has_self || self.dynamic {
                        None
                    } else {
                        Some(parse_quote!(::core::marker::PhantomData))
                    });

                item.default = Some(parse_quote!({ #ident(#(#args),*)}));
            } else if !self.dynamic {
                let default = item.default.take();
                item.default = Some(parse_quote!( {
                    async move {
                        #default
                    }
                }));
            } else {
                let mut default = item.default.take().unwrap();
                rename_self_to_this(&mut default);

                let mut inner = item.clone();
                let mut args = Vec::<Expr>::new();

                for (i, arg) in inner.sig.inputs.iter_mut().enumerate() {
                    match arg {
                        FnArg::Receiver(r) => {
                            let mutability = &r.mutability;
                            let ident = format_ident!("{}Ext", &self.item.ident);
                            if let Some((and_token, lifetime)) = r.reference.clone() {
                                *arg =
                                    parse_quote!(this: #and_token #lifetime #mutability dyn #ident);
                            } else {
                                let ident = format_ident!("SelfTy");
                                *arg = parse_quote!(this: #mutability impl #ident);
                            }
                            args.push(parse_quote!(self));
                        }
                        FnArg::Typed(pat_ty) => {
                            match &*pat_ty.pat {
                                Pat::Ident(pat_ident) => {
                                    // the parameter already has a name -> use it
                                    let ident = &pat_ident.ident;
                                    args.push(parse_quote!(#ident))
                                }
                                _ => {
                                    // the parameter doesn't have a name -> make one and use it
                                    let ident = format_ident!("arg{}", i + 1);
                                    *pat_ty.pat = Pat::Ident(PatIdent {
                                        attrs: Vec::new(),
                                        by_ref: None,
                                        mutability: None,
                                        ident: ident.clone(),
                                        subpat: None,
                                    });
                                    args.push(parse_quote!(#ident))
                                }
                            }
                        }
                    }
                }

                inner.sig.ident = format_ident!("inner");
                inner.default = Some(parse_quote!( {
                    async move {
                        #default
                    }
                }));

                item.default = Some(parse_quote!( {
                    #inner

                    inner(#(#args),*)
                }));
            }
        } else {
            // remove default block
            item.default = None;
        }

        item
    }

    pub fn implement_future(&self) -> ItemImpl {
        let ident = method_struct_name(&self.item, &self.m);
        let return_type = match &self.m.sig.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty.clone(),
        };
        let generics = self.make_generics();
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let trait_ident = &self.item.ident;
        let method_ident = format_ident!("poll_{}", &self.m.sig.ident);
        let args = &self.poll_args;

        let content: Block = if !self.dynamic {
            parse_quote!({
                let this = &mut *self;
                <SelfTy as #trait_ident>:: #method_ident (#(#args.into()),*)
            })
        } else {
            parse_quote!({
                let this = &mut *self;
                #trait_ident:: #method_ident (#(#args.into()),*)
            })
        };

        parse_quote!(
            impl #impl_generics ::core::future::Future for #ident #ty_generics #where_clause {
                type Output = #return_type;

                fn poll(mut self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context) -> ::core::task::Poll<Self::Output> #content
            }
        )
    }

    pub fn future_type_alias(&self) -> ItemType {
        let ident = method_struct_name(&self.item, &self.m);
        let return_type = match &self.m.sig.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, ty) => *ty.clone(),
        };

        let generics = self.make_generics();
        let lifetimes = generics.lifetimes().map(|l| &l.lifetime).cloned();
        let (_, ty_generics, _) = generics.split_for_impl();

        let comment = format!(
            " the future returned by [`{}Ext::{}`]",
            self.item.ident, self.m.sig.ident
        );

        parse_quote!(
            #[doc = #comment]
            type #ident #ty_generics = impl ::core::future::Future<Output = #return_type> #( + #lifetimes)*;
        )
    }

    pub fn is_provided(&self) -> bool {
        self.m.default.is_some()
    }

    fn make_generics(&self) -> Generics {
        let default_lifetime = if self.need_default_lifetime {
            Some(parse_quote!('default_lifetime))
        } else {
            None
        };
        let lifetimes = self
            .item
            .generics
            .lifetimes()
            .chain(self.m.sig.generics.lifetimes())
            .cloned()
            .chain(default_lifetime)
            .map(GenericParam::from);

        let self_generic = if self.has_self || !self.dynamic {
            Some(parse_quote!(SelfTy))
        } else {
            None
        };
        let type_params = self
            .item
            .generics
            .type_params()
            .chain(self.m.sig.generics.type_params())
            .cloned()
            .chain(self_generic)
            .map(GenericParam::from);

        let const_params = self
            .item
            .generics
            .const_params()
            .chain(self.m.sig.generics.const_params())
            .cloned()
            .map(GenericParam::from);

        let params = lifetimes.chain(type_params).chain(const_params);

        let mut item_generics = self.item.generics.clone();
        let mut m_generics = self.m.sig.generics.clone();

        let item_where_clause = item_generics.make_where_clause();
        let m_where_clause = m_generics.make_where_clause();
        m_where_clause
            .predicates
            .iter()
            .cloned()
            .for_each(|i| item_where_clause.predicates.push(i));
        let trait_ident = &self.item.ident;
        if self.has_self || !self.dynamic {
            item_where_clause
                .predicates
                .push(parse_quote!(Self: #trait_ident));
        }
        item_where_clause
            .predicates
            .iter_mut()
            .filter_map(|wp| match wp {
                WherePredicate::Type(t) => Some(&mut t.bounded_ty),
                _ => None,
            })
            .for_each(|t| {
                *t = self.clone().sanize_type(t.clone());
            });

        Generics {
            lt_token: Some(Token![<](Span::call_site())),
            params: Punctuated::from_iter(params),
            gt_token: Some(Token![>](Span::call_site())),
            where_clause: Some(item_where_clause.clone()),
        }
    }

    /// add a receiver parameter
    fn add_receiver(&mut self, receiver: &Receiver) -> Result<()> {
        let mut ty: Type = if !self.dynamic {
            self.has_self = true;
            parse_quote!(SelfTy)
        } else {
            let ident = self.item.ident.clone();
            parse_quote!(dyn #ident)
        };
        if let Some((and_token, lifetime)) = receiver.reference.clone() {
            ty = Type::Reference(TypeReference {
                and_token,
                lifetime,
                mutability: receiver.mutability,
                elem: Box::new(ty),
            })
        } else if self.dynamic {
            return Err(Error::new_spanned(
                receiver,
                "dynamic doesn't work with owned self",
            ));
        }
        self.add_param(ty, true);
        Ok(())
    }

    /// add a function call parameter
    fn add_param(&mut self, ty: Type, param: bool) {
        let ty = self.sanize_type(ty);
        if param {
            let idx = self.fields.len();
            let member = Member::Unnamed(Index {
                index: idx as u32,
                span: Span::call_site(),
            });
            self.poll_args.push(parse_quote!(this.#member));
        }
        self.fields.push(Field {
            attrs: Vec::new(),
            vis: Visibility::Inherited,
            ident: None,
            colon_token: None,
            ty,
        });
    }

    /// change `Self` to `SelfTy` and insert 'default_lifetime for missing lifetimes
    fn sanize_type(&mut self, mut ty: Type) -> Type {
        struct TypeSanitizer<'a> {
            s: &'a mut MethodData,
        }

        impl VisitMut for TypeSanitizer<'_> {
            #[allow(clippy::cmp_owned)]
            fn visit_type_reference_mut(&mut self, i: &mut TypeReference) {
                if i.lifetime.is_none() {
                    self.s.need_default_lifetime = true;
                    i.lifetime = Some(parse_quote!('default_lifetime));
                }
                visit_mut::visit_type_reference_mut(self, i);
            }

            #[allow(clippy::cmp_owned)]
            fn visit_lifetime_mut(&mut self, i: &mut Lifetime) {
                if i.ident.to_string() == "_" {
                    *i = parse_quote!('default_lifetime);
                }
                visit_mut::visit_lifetime_mut(self, i);
            }

            #[allow(clippy::cmp_owned)]
            fn visit_path_segment_mut(&mut self, i: &mut PathSegment) {
                if i.ident.to_string() == "Self" || i.ident == self.s.item.ident {
                    self.s.has_self = true;
                    *i = parse_quote!(SelfTy);
                }
                visit_mut::visit_path_segment_mut(self, i);
            }

            fn visit_type_trait_object_mut(&mut self, _i: &mut TypeTraitObject) {
                // ignore everything in "dyn Trait"
            }
        }

        visit_mut::visit_type_mut(&mut TypeSanitizer { s: self }, &mut ty);

        ty
    }
}

fn rename_self_to_this(b: &mut Block) {
    struct SelfSanitizer;

    impl VisitMut for SelfSanitizer {
        fn visit_item_mut(&mut self, _: &mut Item) {}

        fn visit_expr_path_mut(&mut self, i: &mut ExprPath) {
            if let Some(ident) = i.path.get_ident() {
                if ident == "self" {
                    *i = parse_quote!(this);
                }
            }
            visit_mut::visit_expr_path_mut(&mut SelfSanitizer, i);
        }
    }

    visit_mut::visit_block_mut(&mut SelfSanitizer, b);
}
