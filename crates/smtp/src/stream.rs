//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{io, pin::Pin};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader};

pub struct BufStream<S> {
    pub inner: BufReader<S>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> BufStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            inner: BufReader::new(stream),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner.into_inner()
    }
}

impl<S: AsyncRead + Unpin> AsyncBufRead for BufStream<S> {
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<&[u8]>> {
        Pin::new(&mut self.get_mut().inner).poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut self.get_mut().inner).consume(amt);
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for BufStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for BufStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Pin::new(self.get_mut().inner.get_mut()).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        Pin::new(self.get_mut().inner.get_mut()).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        Pin::new(self.get_mut().inner.get_mut()).poll_shutdown(cx)
    }
}
