#![feature(type_alias_impl_trait)]

use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[test]
fn test_provided() {
    #[async_trait_ext]
    trait Foo {
        async fn bar(&self, i: u32);

        #[async_fn(provided)]
        async fn baz(&self) {
            self.bar(0);
        }
    }

    #[derive(Clone, Copy)]
    struct Quux;

    impl Foo for Quux {
        fn poll_bar(&self, _: u32, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }
    }

    let _ = async {
        Quux.bar(0).await;
        Quux.baz().await;
    };
}

#[async_trait_ext]
pub trait Static {
    async fn method1(&self, val: u32) -> u32;

    #[async_fn(provided)]
    async fn method2(&self, val: u32) -> u32 {
        self.method1(val - 1).await + 1
    }
}

#[async_trait_ext(dynamic)]
pub trait Dynamic {
    async fn method1(&self, val: u32) -> u32;

    #[async_fn(provided)]
    async fn method2(&self, val: u32) -> u32 {
        self.method1(val - 1).await + 1
    }

    #[async_fn(provided)]
    async fn method3<'a>(&'a self, val: &'a mut u32) {
        *val = self.method2(*val).await + 1;
    }
}
