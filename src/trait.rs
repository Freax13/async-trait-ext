use crate::method::MethodData;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, Error, ItemImpl, ItemStruct, ItemTrait, ItemType, Meta, Result, TraitItem};

pub struct TraitData {
    item: ItemTrait,
    methods: Vec<MethodData>,
}

impl TraitData {
    pub fn new(item: ItemTrait, dynamic: bool) -> Result<Self> {
        let mut methods = Vec::with_capacity(item.items.len());
        for m in item
            .items
            .clone()
            .iter_mut()
            .filter_map(|item| {
                if let TraitItem::Method(m) = item {
                    Some(m)
                } else {
                    None
                }
            })
            .filter(|m| m.sig.asyncness.is_some())
        {
            if let Some(default) = m.default.as_ref() {
                let mut found = false;
                let default = default.clone();

                let attrs = m.attrs.clone();
                for (i, attr) in attrs.into_iter().enumerate() {
                    let meta = attr.parse_meta()?;
                    if let Meta::Path(path) = meta {
                        if path.is_ident("provided") {
                            if !cfg!(feature = "provided") {
                                return Err(Error::new_spanned(attr, "enable the 'provided' feature in 'async-trait-ext' to use provided methods"));
                            }
                            found = true;
                            m.attrs.remove(i);
                        }
                    }
                }

                if !found {
                    return Err(Error::new_spanned(
                        default,
                        "provided methods must be marked with #[provided]",
                    ));
                }
            }

            let data = MethodData::new(item.clone(), m.clone(), dynamic)?;
            methods.push(data);
        }

        Ok(TraitData { item, methods })
    }

    pub fn to_tokens(&self) -> TokenStream {
        let pollified = self.pollify_trait();
        let extified = self.extify_trait();
        let future_structs = self.future_structs();
        let future_aliases = self.future_type_aliases();
        let future_impls = self.future_implementations();
        let ext_impl = self.ext_implementations();
        quote!(
            #pollified
            #extified
            #ext_impl
            #(#future_structs)*
            #(#future_impls)*
            #(#future_aliases)*
        )
        .into()
    }

    fn pollify_trait(&self) -> ItemTrait {
        let mut item = self.item.clone();

        // remove async & provided methods
        item.items.retain(|i| {
            if let TraitItem::Method(m) = i {
                m.sig.asyncness.is_none() && m.default.is_none()
            } else {
                true
            }
        });

        // add pollified methods back
        item.items.extend(
            self.methods
                .iter()
                .filter(|m| !m.is_provided())
                .map(MethodData::pollify)
                .map(TraitItem::from),
        );

        item
    }

    fn future_structs(&self) -> impl Iterator<Item = ItemStruct> + '_ {
        self.methods
            .iter()
            .filter(|m| !m.is_provided())
            .map(MethodData::future_struct)
    }

    fn future_type_aliases(&self) -> impl Iterator<Item = ItemType> + '_ {
        self.methods
            .iter()
            .filter(|m| m.is_provided())
            .map(MethodData::future_type_alias)
    }

    fn extify_trait(&self) -> ItemTrait {
        let mut item = self.item.clone();

        // replace doc comments
        let comment = format!(" the extension trait for [`{}`]", self.item.ident);
        item.attrs = vec![parse_quote!(#[doc = #comment])];

        // add trait bounds
        let ident = &self.item.ident;
        item.supertraits.push(parse_quote!(#ident));

        // add "Ext" to ident
        item.ident = format_ident!("{}Ext", &item.ident);

        // remove all items
        item.items.clear();

        // add pollified methods back
        item.items.extend(
            self.methods
                .iter()
                .map(|m| m.extify(false))
                .map(TraitItem::from),
        );

        item
    }

    fn future_implementations(&self) -> impl Iterator<Item = ItemImpl> + '_ {
        self.methods
            .iter()
            .filter(|m| !m.is_provided())
            .map(MethodData::implement_future)
    }

    fn ext_implementations(&self) -> ItemImpl {
        let ident = &self.item.ident;
        let ext_ident = format_ident!("{}Ext", ident);

        let methods = self.methods.iter().map(|m| m.extify(true));

        parse_quote!(
            impl<SelfTy: #ident> #ext_ident for SelfTy {
                #(#methods)*
            }
        )
    }
}
