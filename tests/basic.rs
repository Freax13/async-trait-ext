use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[test]
fn test_empty() {
    #[async_trait_ext]
    trait Foo {
        async fn bar();
    }

    struct Baz;

    impl Foo for Baz {
        fn poll_bar(_: &mut Context) -> Poll<()> {
            unimplemented!()
        }
    }

    let _ = async {
        Baz::bar().await;
    };
}

#[test]
fn test_receiver() {
    #[async_trait_ext]
    trait Foo {
        async fn bar(self)
        where
            Self: Copy + Unpin;
        async fn baz(&self);
        async fn qux(&mut self);
    }

    #[derive(Clone, Copy)]
    struct Quux;

    impl Foo for Quux {
        fn poll_bar(self, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }

        fn poll_baz(&self, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }

        fn poll_qux(&mut self, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }
    }

    let _ = async {
        Quux.bar().await;
        Quux.baz().await;
        Quux.qux().await;
    };
}

#[async_trait_ext]
trait ReceiverWithData {
    async fn owned(self, data: &[u8])
    where
        Self: Copy + Unpin;
    async fn borrowed<'a>(&'a self, data: &'a [u8]);
    async fn mutably_borrowed<'a>(&'a mut self, data: &'a [u8]);
}

#[test]
fn test_without_receiver() {
    #[async_trait_ext]
    trait Foo {
        async fn bar(_: i32);
        async fn baz(_: Self, _: i32)
        where
            Self: Copy + Unpin;
    }

    #[derive(Clone, Copy)]
    struct Qux;

    impl Foo for Qux {
        fn poll_bar(_: i32, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }

        fn poll_baz(_: Self, _: i32, _: &mut Context) -> Poll<()> {
            unimplemented!()
        }
    }

    let _ = async {
        Qux::bar(1).await;
        Qux::baz(Qux, 1).await;
    };
}
