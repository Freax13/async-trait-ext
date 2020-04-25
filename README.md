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

The tests contain also some examples on how the traits can be used.

## Disclaimer
It's very likely that the macro still has some bugs (especially concerning things like generics)