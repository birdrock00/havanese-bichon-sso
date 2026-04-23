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


use std::pin::Pin;
use std::task::{Context, Poll};
// use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::imap::session::SessionStream;

pub struct StatsWrapper<T> {
    inner: T,
}

impl<T> StatsWrapper<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for StatsWrapper<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        // if let Poll::Ready(Ok(())) = &result {
        //     // let bytes_read = buf.filled().len() - before;
        // }
        result
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for StatsWrapper<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        // if let Poll::Ready(Ok(bytes_written)) = &result {
        // }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl<T: SessionStream> SessionStream for StatsWrapper<T> {
    // fn set_read_timeout(&mut self, timeout: Option<Duration>) {
    //     self.inner.set_read_timeout(timeout);
    // }
}

impl<T: SessionStream> std::fmt::Debug for StatsWrapper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatsWrapper")
            .field("inner", &self.inner)
            .finish()
    }
}
