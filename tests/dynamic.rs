use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[test]
fn test_dynamic() {
    #[async_trait_ext(dynamic)]
    pub trait Foo {
        async fn bar(&mut self);
    }

    struct Baz;

    impl Foo for Baz {
        fn poll_bar<'a>(&'a mut self, _: &mut Context) -> Poll<()> {
            Poll::Ready(())
        }
    }

    let dynamic: &mut dyn FooExt = &mut Baz;

    let _ = async {
        dynamic.bar().await;
    };
}
