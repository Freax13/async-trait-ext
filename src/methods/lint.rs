use crate::input::{MethodInput, TraitInput};
use macro_compose::{Collector, Context, Lint};
use syn::{Error, FnArg, ItemTrait, TraitItem};

pub struct MethodAttrLint;

impl Lint<ItemTrait> for MethodAttrLint {
    fn lint(&self, input: &ItemTrait, c: &mut Collector) {
        let attrs = TraitInput::from(input.attrs.as_slice());

        for item in input.items.iter() {
            if let TraitItem::Method(m) = item {
                if attrs.dynamic.is_some() {
                    if let Some(FnArg::Receiver(r)) = m.sig.inputs.first() {
                        if r.reference.is_none() {
                            c.error(Error::new_spanned(
                                r,
                                "dynamic traits can't receive owned self",
                            ));
                        }
                    }
                }

                let mut subcontext = Context::new_by_ref(c, &m.attrs);
                if subcontext.lint(MethodInput::lint()) {
                    let input = MethodInput::from(m.attrs.as_slice());
                    if input.provided.is_none() && m.default.is_some() {
                        c.error(Error::new_spanned(
                            &m.default,
                            "provided methods must be marked with #[async_fn(provided)]",
                        ));
                    } else if input.provided.is_some() && m.default.is_none() {
                        c.error(Error::new_spanned(
                            &m,
                            "provided methods must have a default block",
                        ));
                    }
                }
            }
        }
    }
}
