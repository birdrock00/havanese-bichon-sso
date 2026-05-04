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

use deunicode::deunicode;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

#[derive(Clone, Debug, Default)]
pub struct DeunicodeFilter;

impl TokenFilter for DeunicodeFilter {
    type Tokenizer<T: Tokenizer> = DeunicodeFilterWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> Self::Tokenizer<T> {
        DeunicodeFilterWrapper { inner: tokenizer }
    }
}

#[derive(Clone, Debug)]
pub struct DeunicodeFilterWrapper<T> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for DeunicodeFilterWrapper<T> {
    type TokenStream<'a> = DeunicodeTokenStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        DeunicodeTokenStream {
            inner: self.inner.token_stream(text),
        }
    }
}

pub struct DeunicodeTokenStream<S> {
    inner: S,
}

impl<S: TokenStream> TokenStream for DeunicodeTokenStream<S> {
    fn advance(&mut self) -> bool {
        if !self.inner.advance() {
            return false;
        }
        let token: &mut Token = self.inner.token_mut();
        // Only allocate a new String when the text actually contains
        // non-ASCII characters to avoid unnecessary overhead.
        if !token.text.is_ascii() {
            token.text = deunicode(&token.text);
        }
        true
    }

    fn token(&self) -> &Token {
        self.inner.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.inner.token_mut()
    }
}
