#![feature(type_alias_impl_trait)]

#[test]
fn test_mut_pattern() {
    use async_trait_ext::async_trait_ext;

    #[async_trait_ext(dynamic)]
    pub trait Foo {
        async fn bar(&self, a: u32);

        #[async_fn(provided)]
        async fn baz(&self, mut a: u32) -> u32 {
            a += 1;
            a
        }
    }

    #[async_trait_ext]
    pub trait Foo2 {
        async fn bar(&self, a: u32);

        #[async_fn(provided)]
        async fn baz(&self, mut a: u32) -> u32 {
            a += 1;
            a
        }
    }
}
