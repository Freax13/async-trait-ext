#![feature(type_alias_impl_trait)]

use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[test]
fn test_provided() {
    #[async_trait_ext]
    trait Foo {
        async fn bar(&self, i: u32);

        #[provided]
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
