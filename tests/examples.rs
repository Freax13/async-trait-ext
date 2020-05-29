#![feature(type_alias_impl_trait)]

use async_trait_ext::async_trait_ext;
use std::io::Result;

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

pub struct LockGuard;

#[async_trait_ext]
pub trait Lock {
    async fn lock(&self) -> Result<LockGuard>;
}

#[async_trait_ext(dynamic)]
pub trait Write {
    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> Result<usize>;
}

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