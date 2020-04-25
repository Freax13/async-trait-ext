#[test]
fn test_visibility() {
    mod inner {
        use async_trait_ext::async_trait_ext;

        #[async_trait_ext]
        pub trait Foo {
            async fn bar(&self);
        }
    }

    #[allow(unused_imports)]
    use inner::{Foo, FooExt};
}
