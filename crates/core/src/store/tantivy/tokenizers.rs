//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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

use tantivy::tokenizer::*;
use whichlang::{detect_language, Lang};

use crate::store::tantivy::filter::DeunicodeFilter;

#[derive(Clone)]
pub struct EuroTokenizer {
    arabic: TextAnalyzer,
    dutch: TextAnalyzer,
    english: TextAnalyzer,
    french: TextAnalyzer,
    german: TextAnalyzer,
    italian: TextAnalyzer,
    portuguese: TextAnalyzer,
    russian: TextAnalyzer,
    spanish: TextAnalyzer,
    swedish: TextAnalyzer,
    turkish: TextAnalyzer,
    default: TextAnalyzer,
}

impl EuroTokenizer {
    pub fn new() -> Self {
        fn build(lang: Language) -> TextAnalyzer {
            TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(DeunicodeFilter)
                .filter(Stemmer::new(lang))
                .build()
        }

        Self {
            arabic: build(Language::Arabic),
            dutch: build(Language::Dutch),
            english: build(Language::English),
            french: build(Language::French),
            german: build(Language::German),
            italian: build(Language::Italian),
            portuguese: build(Language::Portuguese),
            russian: build(Language::Russian),
            spanish: build(Language::Spanish),
            swedish: build(Language::Swedish),
            turkish: build(Language::Turkish),
            default: TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .build(),
        }
    }
}

impl Tokenizer for EuroTokenizer {
    type TokenStream<'a> = BoxTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        match detect_language(text) {
            Lang::Ara => self.arabic.token_stream(text),
            Lang::Eng => self.english.token_stream(text),
            Lang::Fra => self.french.token_stream(text),
            Lang::Deu => self.german.token_stream(text),
            Lang::Spa => self.spanish.token_stream(text),
            Lang::Nld => self.dutch.token_stream(text),
            Lang::Por => self.portuguese.token_stream(text),
            Lang::Rus => self.russian.token_stream(text),
            Lang::Swe => self.swedish.token_stream(text),
            Lang::Tur => self.turkish.token_stream(text),
            Lang::Ita => self.italian.token_stream(text),
            _ => self.default.token_stream(text), // fallback
        }
    }
}
