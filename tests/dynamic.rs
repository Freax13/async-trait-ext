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

#[async_trait_ext(dynamic)]
pub trait Foo {
    async fn method1(&self);
    async fn method2(&mut self);
    async fn method3(&self, a: u32);
    async fn method4(&mut self, a: u32);
    async fn method5<'a>(&'a self, a: &'a [u8]);
    async fn method6<'a>(&'a mut self, a: &'a [u8]);
}
