# Async trait methods with polling and extension traits
Rust's trait system doesn't allow async methods in traits. The `async-trait` crate solves this by returning boxed futures.

For most cases `async-trait` works pretty well, however some low-level traits can't afford to do a heap allocation everytime they're called, so other crates like `tokio` started using "extension traits". They basically converted `Read::read` to `AsyncRead::poll_read` which uses polling and provided a new method `AsyncReadExt::read` that returns a future that internally uses `poll_read` (`AsyncReadExt` is automatically implemented for all `AsyncRead`). Since `poll_read` is synchronous, the trait doesn't contain any async methods and can be compiled by Rust. All implementors just have to implement `poll_read` instead of `read`.

Writing extension traits by hand can be very tedious, so the `async_trait_ext` macro can be used take care of writing all that boilerplate code.

## Dynamic trait objects
For a trait like 
```rust
#[async_trait_ext]
trait AsyncRead {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;
}
```
`async_trait_ext` by default generates a struct like
```rust
struct AsyncReadRead<'a, SelfTy: AsyncRead>(&'a mut SelfTy, &'a mut [u8]);
```
to implement the future for `AsyncRead::read`. This struct contains Self so it needs to be sized.

However it's also possible to optin to dynamic types by using `async_trait_ext(dynamic)`.
```rust
#[async_trait_ext(dynamic)]
trait AsyncRead {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;
}
```
generates
```rust
struct AsyncReadRead<'a>(&'a mut dyn AsyncRead, &'a mut [u8]);
```
which doesn't need to be sized.

### Provided functions
Sometimes it's usefully to have provided functions for a trait. Marking functions with the `provided` attribute moves them into the extension trait.

Unfortunatly the `type_alias_impl_trait` feature on nightly is required to name the type of an async block. Because of that the `provided` attribute can only be used with the `provided` feature of `async-trait-ext` (not enabled by default).

Example
```rust
#![feature(type_alias_impl_trait)]

#[async_trait_ext(dynamic)]
trait AsyncRead {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;
    
    #[async_fn(provided)]
    async fn read_u8<'a>(&'a mut self) -> Result<u8> {
        let mut buf = [0];
        self.read(&mut buf).await?;
        Ok(buf[0])
    }
}
```

## Examples
### Non-dynamic
```rust
#[async_trait_ext]
pub trait Lock {
    async fn lock(&self) -> Result<LockGuard>;
}
```
expands to
```rust
pub trait Lock {
    fn poll_lock(&self, ctx: &mut ::core::task::Context) -> ::core::task::Poll<Result<LockGuard>>;
}

/// the future returned by [`LockExt::lock`]
pub struct LockLock<'__default_lifetime, __Self: Lock>(
    &'__default_lifetime __Self,
    ::core::marker::PhantomData<*const __Self>,
    ::core::marker::PhantomData<&'__default_lifetime ()>,
);

impl<'__default_lifetime, __Self: Lock> ::core::future::Future
    for LockLock<'__default_lifetime, __Self>
{
    type Output = Result<LockGuard>;
    
    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        <__Self as Lock>::poll_lock(this.0.into(), cx)
    }
}

pub trait LockExt: Lock + ::core::marker::Sized {
    fn lock(&self) -> LockLock<'_, Self>;
}

impl<__IMPL: Lock> LockExt for __IMPL {
    fn lock(&self) -> LockLock<'_, Self> {
        LockLock(
            self,
            ::core::marker::PhantomData,
            ::core::marker::PhantomData,
        )
    }
}
```
### Dynamic
```rust
#[async_trait_ext(dynamic)]
pub trait Write {
    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> Result<usize>;
}
```
expands to
```rust
pub trait Write {
    fn poll_write<'a>(
        &'a mut self,
        buf: &'a [u8],
        ctx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Result<usize>>;
}

/// the future returned by [`WriteExt::write`]
pub struct WriteWrite<'a>(
    &'a mut dyn Write,
    &'a [u8],
    ::core::marker::PhantomData<&'a ()>,
);

impl<'a> ::core::future::Future for WriteWrite<'a> {
    type Output = Result<usize>;

    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        Write::poll_write(this.0.into(), this.1.into(), cx)
    }
}

pub trait WriteExt: Write {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> WriteWrite<'a>;
}

impl<__IMPL: Write> WriteExt for __IMPL {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> WriteWrite<'a> {
        WriteWrite(self, buf, ::core::marker::PhantomData)
    }
}
```

### Provided methods
```rust
#![feature(type_alias_impl_trait)]

#[async_trait_ext]
pub trait ReadStatic {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;

    #[async_fn(provided)]
    async fn read_until<'a>(&'a mut self, byte: u8, mut buf: &'a mut [u8]) -> Result<usize> {
        let mut b = [0];
        let mut bytes_read = 0;

        while !buf.is_empty() {
            match self.read(&mut b).await? {
                1 if b[0] != byte => {
                    bytes_read += 1;
                    buf[0] = b[0];
                    buf = &mut buf[1..];
                }
                _ => break,
            }
        }

        Ok(bytes_read)
    }
}
```
expands to
```rust
#![feature(type_alias_impl_trait)]

pub trait ReadStatic {
    fn poll_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
        ctx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Result<usize>>;
}

/// the future returned by [`ReadStaticExt::read`]
pub struct ReadStaticRead<'a, __Self: ReadStatic>(
    &'a mut __Self,
    &'a mut [u8],
    ::core::marker::PhantomData<*const __Self>,
    ::core::marker::PhantomData<&'a ()>,
);

impl<'a, __Self: ReadStatic> ::core::future::Future for ReadStaticRead<'a, __Self> {
    type Output = Result<usize>;

    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        <__Self as ReadStatic>::poll_read(this.0.into(), this.1.into(), cx)
    }
}

/// the future returned by [`ReadStaticExt::read_until`]
pub type ReadStaticReadUntil<'a, __Self: ReadStatic> =
    impl ::core::future::Future<Output = Result<usize>>;

pub trait ReadStaticExt: ReadStatic + ::core::marker::Sized {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadStaticRead<'a, Self>;
    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadStaticReadUntil<'a, Self>;
}

impl<__IMPL: ReadStatic> ReadStaticExt for __IMPL {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadStaticRead<'a, Self> {
        ReadStaticRead(
            self,
            buf,
            ::core::marker::PhantomData,
            ::core::marker::PhantomData,
        )
    }

    fn read_until<'a>(
        &'a mut self,
        byte: u8,
        mut buf: &'a mut [u8],
    ) -> ReadStaticReadUntil<'a, Self> {
        async move {
            {
                let mut b = [0];
                let mut bytes_read = 0;
                while !buf.is_empty() {
                    match self.read(&mut b).await? {
                        1 if b[0] != byte => {
                            bytes_read += 1;
                            buf[0] = b[0];
                            buf = &mut buf[1..];
                        }
                        _ => break,
                    }
                }
                Ok(bytes_read)
            }
        }
    }
}
```

### Provided methods + dynamic
```rust
#![feature(type_alias_impl_trait)]

#[async_trait_ext(dynamic)]
pub trait ReadDynamic {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;

    #[async_fn(provided)]
    async fn read_until<'a>(&'a mut self, byte: u8, mut buf: &'a mut [u8]) -> Result<usize> {
        let mut b = [0];
        let mut bytes_read = 0;

        while !buf.is_empty() {
            match self.read(&mut b).await? {
                1 if b[0] != byte => {
                    bytes_read += 1;
                    buf[0] = b[0];
                    buf = &mut buf[1..];
                }
                _ => break,
            }
        }

        Ok(bytes_read)
    }
}
```
expands to
```rust
#![feature(type_alias_impl_trait)]

pub trait ReadDynamic {
    fn poll_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
        ctx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Result<usize>>;
}

/// the future returned by [`ReadDynamicExt::read`]
pub struct ReadDynamicRead<'a>(
    &'a mut dyn ReadDynamic,
    &'a mut [u8],
    ::core::marker::PhantomData<&'a ()>,
);

impl<'a> ::core::future::Future for ReadDynamicRead<'a> {
    type Output = Result<usize>;
    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        ReadDynamic::poll_read(this.0.into(), this.1.into(), cx)
    }
}

/// the future returned by [`ReadDynamicExt::read_until`]
pub type ReadDynamicReadUntil<'a> = impl ::core::future::Future<Output = Result<usize>>;

pub trait ReadDynamicExt: ReadDynamic {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadDynamicRead<'a>;
    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadDynamicReadUntil<'a>;
}

impl<__IMPL: ReadDynamic> ReadDynamicExt for __IMPL {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadDynamicRead<'a> {
        ReadDynamicRead(self, buf, ::core::marker::PhantomData)
    }

    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadDynamicReadUntil<'a> {
        async fn fn_impl<'a>(
            this: &mut dyn ReadDynamicExt,
            byte: u8,
            mut buf: &'a mut [u8],
        ) -> Result<usize> {
            {
                let mut b = [0];
                let mut bytes_read = 0;
                while !buf.is_empty() {
                    match this.read(&mut b).await? {
                        1 if b[0] != byte => {
                            bytes_read += 1;
                            buf[0] = b[0];
                            buf = &mut buf[1..];
                        }
                        _ => break,
                    }
                }
                Ok(bytes_read)
            }
        }
        fn_impl(self, byte, buf)
    }
}
```

The tests contain also some examples on how the traits can be used.