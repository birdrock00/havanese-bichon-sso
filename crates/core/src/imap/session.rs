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
use tokio::io::{AsyncRead, AsyncWrite, BufWriter};
use tokio_io_timeout::TimeoutStream;

pub trait SessionStream: AsyncRead + AsyncWrite + Unpin + Send + Sync + std::fmt::Debug {
    //  Change the read timeout on the session stream.
    // fn set_read_timeout(&mut self, timeout: Option<Duration>);
}

impl SessionStream for Box<dyn SessionStream> {
    // fn set_read_timeout(&mut self, timeout: Option<Duration>) {
    //     self.as_mut().set_read_timeout(timeout);
    // }
}

impl<T: SessionStream> SessionStream for tokio_rustls::client::TlsStream<T> {
    // fn set_read_timeout(&mut self, timeout: Option<Duration>) {
    //     self.get_mut().0.set_read_timeout(timeout);
    // }
}

impl<T: SessionStream> SessionStream for BufWriter<T> {
    // fn set_read_timeout(&mut self, timeout: Option<Duration>) {
    //     self.get_mut().set_read_timeout(timeout);
    // }
}
impl<T: AsyncRead + AsyncWrite + Send + Sync + std::fmt::Debug> SessionStream
    for Pin<Box<TimeoutStream<T>>>
{
    // fn set_read_timeout(&mut self, timeout: Option<Duration>) {
    //     self.as_mut().set_read_timeout_pinned(timeout);
    // }
}
