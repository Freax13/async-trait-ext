use async_trait_ext::async_trait_ext;

#[test]
fn test_sync() {
    #[async_trait_ext]
    trait Foo {
        fn bar(self)
        where
            Self: Copy + Unpin;
        fn baz(&self);
        fn qux(&mut self);
    }

    #[derive(Clone, Copy)]
    struct Quux;

    impl Foo for Quux {
        fn bar(self) {
            unimplemented!()
        }

        fn baz(&self) {
            unimplemented!()
        }

        fn qux(&mut self) {
            unimplemented!()
        }
    }

    let _ = async {
        Quux.bar();
        Quux.baz();
        Quux.qux();
    };
}
