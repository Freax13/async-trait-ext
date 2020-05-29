use async_trait_ext::async_trait_ext;
use std::task::{Context, Poll};

#[async_trait_ext]
pub trait Foo {
    async fn method1(self)
    where
        Self: Copy + Unpin;
    async fn method2(&self);
    async fn method3(&mut self);
    async fn method4();

    #[allow(clippy::needless_lifetimes)]
    async fn method5<'a, A: Clone>(self, a: &'a A)
    where
        Self: Copy + Unpin;
    async fn method6<'a, A: Clone>(&self, a: &'a A);
    async fn method7<'a, A: Clone>(&mut self, a: &'a A);
    #[allow(clippy::needless_lifetimes)]
    async fn method8<'a, A: Clone>(a: &'a A);

    async fn method9<A: Clone>(self)
    where
        Self: Copy + Unpin;
    async fn method10<A: Clone>(&self);
    async fn method11<A: Clone>(&mut self);
    async fn method12<A: Clone>();

    async fn method13<A: Clone>(self) -> A
    where
        Self: Copy + Unpin;
    async fn method14<A: Clone>(&self) -> A;
    async fn method15<A: Clone>(&mut self) -> A;
    async fn method16<A: Clone>() -> A;

    #[allow(clippy::extra_unused_lifetimes)]
    async fn method17<'a>(self)
    where
        Self: Copy + Unpin;
    #[allow(clippy::extra_unused_lifetimes)]
    async fn method18<'a>(&self);
    #[allow(clippy::extra_unused_lifetimes)]
    async fn method19<'a>(&mut self);
    #[allow(clippy::extra_unused_lifetimes)]
    async fn method20<'a>();

    fn method21();
    #[allow(clippy::extra_unused_lifetimes)]
    fn method22<'a>();
}

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
