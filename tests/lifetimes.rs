use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[test]
fn test_lifetimes() {
    #[async_trait_ext]
    trait Foo {
        async fn bar<'i>(&self, _: &'i i32);
    }

    #[derive(Clone, Copy)]
    struct Baz;

    impl Foo for Baz {
        fn poll_bar<'i>(&'i self, _: &'i i32, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }
    }

    let _ = async {
        Baz.bar(&1).await;
    };
}
