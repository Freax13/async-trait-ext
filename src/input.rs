use macro_input::MacroInput;
#[derive(MacroInput, Debug)]
#[macro_input(rename = "async_trait_ext")]
pub struct TraitInput {
    pub dynamic: Option<()>,
}

#[derive(MacroInput, Debug)]
#[macro_input(rename = "async_fn")]
pub struct MethodInput {
    pub provided: Option<()>,
}
