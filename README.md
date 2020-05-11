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
    
    #[provided]
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
    async fn lock<'a>(&'a self) -> Result<LockGuard>;
}
```
expands to
```rust
pub trait Lock {
    fn poll_lock<'a>(
        &'a self,
        cx: &mut core::task::Context,
    ) -> core::task::Poll<Result<LockGuard>>;
}

/// the extension trait for [`Lock`]
pub trait LockExt: Lock {
    fn lock<'a>(&'a self) -> LockLock<'a, Self>
    where
        Self: Sized;
}

impl<SelfTy: Lock> LockExt for SelfTy {
    fn lock<'a>(&'a self) -> LockLock<'a, Self>
    where
        Self: Sized,
    {
        LockLock(self)
    }
}

/// the future returned by [`LockExt::lock`]
pub struct LockLock<'a, SelfTy>(&'a SelfTy)
where
    SelfTy: Lock;

impl<'a, SelfTy> core::future::Future for LockLock<'a, SelfTy>
where
    SelfTy: Lock,
{
    type Output = Result<LockGuard>;
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context,
    ) -> core::task::Poll<Self::Output> {
        let this = &mut *self;
        <SelfTy as Lock>::poll_lock(this.0.into(), cx.into())
    }
}
```
### Dynamic
```rust
#[async_trait_ext(dynamic)]
pub trait Read {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;
}
```
expands to
```rust
pub trait Read {
    fn poll_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
        cx: &mut core::task::Context,
    ) -> core::task::Poll<Result<usize>>;
}

/// the extension trait for [`Read`]
pub trait ReadExt: Read {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a>;
}

impl<SelfTy: Read> ReadExt for SelfTy {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a> {
        ReadRead(self, buf)
    }
}

/// the future returned by [`ReadExt::read`]
pub struct ReadRead<'a>(&'a mut dyn Read, &'a mut [u8]);

impl<'a> core::future::Future for ReadRead<'a> {
    type Output = Result<usize>;
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context,
    ) -> core::task::Poll<Self::Output> {
        let this = &mut *self;
        Read::poll_read(this.0.into(), this.1.into(), cx.into())
    }
}
```

### Provided methods
```rust
#![feature(type_alias_impl_trait)]

#[async_trait_ext]
pub trait Read {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;

    #[provided]
    async fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> Result<usize> {
        let mut buf = buf;

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

pub trait Read {
    fn poll_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Result<usize>>;
}

/// the extension trait for [`Read`]
pub trait ReadExt: Read {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a, Self>
    where
        Self: Sized;
    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadReadUntil<'a, Self>
    where
        Self: Sized;
}

impl<SelfTy: Read> ReadExt for SelfTy {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a, Self>
    where
        Self: Sized,
    {
        ReadRead(self, buf)
    }

    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadReadUntil<'a, Self>
    where
        Self: Sized,
    {
        async move {
            {
                let mut buf = buf;
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

/// the future returned by [`ReadExt::read`]
pub struct ReadRead<'a, SelfTy>(&'a mut SelfTy, &'a mut [u8])
where
    SelfTy: Read;

impl<'a, SelfTy> ::core::future::Future for ReadRead<'a, SelfTy>
where
    SelfTy: Read,
{
    type Output = Result<usize>;
    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        <SelfTy as Read>::poll_read(this.0.into(), this.1.into(), cx.into())
    }
}

/// the future returned by [`ReadExt::read_until`]
type ReadReadUntil<'a, SelfTy> = impl ::core::future::Future<Output = Result<usize>> + 'a;
```

### Provided methods + dynamic
```rust
#![feature(type_alias_impl_trait)]

#[async_trait_ext(dynamic)]
pub trait Read {
    async fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Result<usize>;

    #[provided]
    async fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> Result<usize> {
        let mut buf = buf;

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

pub trait Read {
    fn poll_read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Result<usize>>;
}

/// the extension trait for [`Read`]
pub trait ReadExt: Read {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a>;
    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadReadUntil<'a>;
}

impl<SelfTy: Read> ReadExt for SelfTy {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadRead<'a> {
        ReadRead(self, buf)
    }

    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut [u8]) -> ReadReadUntil<'a> {
        fn inner<'a>(this: &'a mut dyn ReadExt, byte: u8, buf: &'a mut [u8]) -> ReadReadUntil<'a> {
            async move {
                {
                    let mut buf = buf;
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
        }
        inner(self, byte, buf)
    }
}

/// the future returned by [`ReadExt::read`]
pub struct ReadRead<'a>(&'a mut dyn Read, &'a mut [u8]);

impl<'a> ::core::future::Future for ReadRead<'a> {
    type Output = Result<usize>;
    fn poll(
        mut self: ::core::pin::Pin<&mut Self>,
        cx: &mut ::core::task::Context,
    ) -> ::core::task::Poll<Self::Output> {
        let this = &mut *self;
        Read::poll_read(this.0.into(), this.1.into(), cx.into())
    }
}

/// the future returned by [`ReadExt::read_until`]
type ReadReadUntil<'a> = impl ::core::future::Future<Output = Result<usize>> + 'a;
```

The tests contain also some examples on how the traits can be used.

## Disclaimer
It's very likely that the macro still has some bugs (especially concerning things like generics)