use crate::method::MethodData;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, ItemImpl, ItemStruct, ItemTrait, Result, TraitItem};

pub struct TraitData<'i> {
    item: &'i ItemTrait,
    methods: Vec<MethodData<'i>>,
}

impl<'i> TraitData<'i> {
    pub fn new(item: &'i ItemTrait, dynamic: bool) -> Result<Self> {
        let mut methods = Vec::with_capacity(item.items.len());
        for m in item
            .items
            .iter()
            .filter_map(|item| {
                if let TraitItem::Method(m) = item {
                    Some(m)
                } else {
                    None
                }
            })
            .filter(|m| m.sig.asyncness.is_some())
        {
            let data = MethodData::new(item, m, dynamic)?;
            methods.push(data);
        }

        Ok(TraitData { item, methods })
    }

    pub fn to_tokens(&self) -> TokenStream {
        let pollified = self.pollify_trait();
        let extified = self.extify_trait();
        let future_structs = self.future_structs();
        let future_impls = self.future_implementations();
        let ext_impl = self.ext_implementations();
        quote!(
            #pollified
            #extified
            #ext_impl
            #(#future_structs)*
            #(#future_impls)*
        )
        .into()
    }

    fn pollify_trait(&self) -> ItemTrait {
        let mut item = self.item.clone();

        // remove async methods
        item.items.retain(|i| {
            if let TraitItem::Method(m) = i {
                m.sig.asyncness.is_none()
            } else {
                true
            }
        });

        // add pollified methods back
        item.items.extend(
            self.methods
                .iter()
                .map(MethodData::pollify)
                .map(TraitItem::from),
        );

        item
    }

    fn future_structs(&self) -> impl Iterator<Item = ItemStruct> + '_ {
        self.methods.iter().map(MethodData::future_struct)
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
        self.methods.iter().map(MethodData::implement_future)
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
