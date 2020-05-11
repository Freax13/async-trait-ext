#![feature(type_alias_impl_trait)]

#[test]
fn test_mut_pattern() {
    use async_trait_ext::async_trait_ext;

    #[async_trait_ext(dynamic)]
    pub trait Foo {
        #[provided]
        async fn bar(&self, mut a: u32) {
            a += 1;
        }
    }
}
