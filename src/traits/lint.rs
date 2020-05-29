use crate::input::TraitInput;
use macro_compose::{Collector, Context, Lint};
use syn::ItemTrait;

pub struct AttributeLint;

impl Lint<ItemTrait> for AttributeLint {
    fn lint(&self, input: &ItemTrait, c: &mut Collector) {
        let mut ctx = Context::new_by_ref(c, &input.attrs);
        ctx.lint(TraitInput::lint());
    }
}
