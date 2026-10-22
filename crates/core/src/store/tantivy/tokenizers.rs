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

use deunicode::deunicode;
use tantivy::query::{BooleanQuery, BoostQuery, EmptyQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::tokenizer::*;
use tantivy::{Index, Term};

use crate::store::tantivy::filter::DeunicodeFilter;

/// Deterministic pipeline: SimpleTokenizer → RemoveLongFilter(40) → LowerCaser
/// → DeunicodeFilter.
///
/// No stemming and no language detection: the same word produces the same token
/// whether it arrives as part of a whole message or as a single query term, so
/// the index side and the query side always agree (the root fix for #361).
/// CJK is out of scope (deunicode transliterates; there is no CJK segmentation).
#[derive(Clone)]
pub struct EuroTokenizer {
    analyzer: TextAnalyzer,
}

impl EuroTokenizer {
    pub fn new() -> Self {
        Self {
            analyzer: TextAnalyzer::builder(SimpleTokenizer::default())
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .filter(DeunicodeFilter)
                .build(),
        }
    }
}

impl Default for EuroTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for EuroTokenizer {
    type TokenStream<'a> = BoxTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        self.analyzer.token_stream(text)
    }
}

/// Trigram analyzer: NgramTokenizer(3,3) → LowerCaser → DeunicodeFilter.
///
/// The normalization order must match [`trigrams`] exactly (window first, then
/// lowercase, then deunicode), or the query-side grams will not line up with the
/// terms written at index time.
pub fn tri_analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(NgramTokenizer::new(3, 3, false).expect("ngram 3,3 is valid"))
        .filter(LowerCaser)
        .filter(DeunicodeFilter)
        .build()
}

/// Compute the 3-grams of a query term exactly as [`tri_analyzer`] would
/// (lowercase + deunicode). Fewer than 3 characters yields an empty vector — the
/// caller falls back to a pure word-level query in that case.
pub fn trigrams(term: &str) -> Vec<String> {
    let chars: Vec<char> = term.chars().collect();
    let mut out = Vec::new();
    if chars.len() < 3 {
        return out;
    }
    for w in chars.windows(3) {
        let g: String = w.iter().collect();
        let g = g.to_lowercase();
        let g = if g.is_ascii() { g } else { deunicode(&g) };
        out.push(g);
    }
    out
}

/// Thresholded trigram subquery on a single field (leave-one-out):
/// `Should[ all grams Must, one-gram-missing Must × n ]` — i.e. at least
/// (n-1)/n grams must be present.
///
/// Tolerates one level of inflection (kroatiens↔kroatien) while rejecting hits
/// driven by a single common gram ("ers" in "Vers"). Dumping every gram into
/// BM25 as `Should` collapses precision@10, so the threshold is mandatory.
fn tri_field_subquery(field: Field, term: &str) -> Box<dyn Query> {
    let grams = trigrams(term);
    if grams.is_empty() {
        return Box::new(EmptyQuery);
    }
    let gram_terms: Vec<Term> = grams
        .iter()
        .map(|g| Term::from_field_text(field, g))
        .collect();

    let must_clauses = |skip: Option<usize>| -> Vec<(Occur, Box<dyn Query>)> {
        gram_terms
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != skip)
            .map(|(_, t)| {
                (
                    Occur::Must,
                    Box::new(TermQuery::new(t.clone(), IndexRecordOption::Basic)) as Box<dyn Query>,
                )
            })
            .collect()
    };

    let mut variants: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    // Variant 0: every gram Must (pure substring semantics).
    variants.push((Occur::Should, Box::new(BooleanQuery::new(must_clauses(None)))));
    // Variants 1..n: one gram missing (one level of inflection).
    //
    // Only worth generating from 3 grams up. With exactly 2 grams (a 4-character
    // term) a "one gram missing" variant collapses to a single gram, i.e. "either
    // gram matches" — for `haus` that would match `Mauser` on the shared `aus`
    // gram, and `haus` is not even a substring of `mauser`. Requiring both grams
    // is precisely the 4-character substring test, so nothing real is lost.
    if gram_terms.len() >= 3 {
        for skip in 0..gram_terms.len() {
            let clauses = must_clauses(Some(skip));
            if !clauses.is_empty() {
                variants.push((Occur::Should, Box::new(BooleanQuery::new(clauses))));
            }
        }
    }
    Box::new(BooleanQuery::new(variants))
}

/// Fuzzy subquery for a query text over the given trigram fields.
///
/// Terms are **AND**-ed and fields are OR-ed, mirroring the word-level parser
/// under conjunction: every term must match in at least one of the fields.
///
/// If any term is too short to produce grams (fewer than 3 characters) the
/// trigram layer cannot express that conjunction faithfully — an empty clause
/// would make the whole conjunct false — so it returns `EmptyQuery` and the word
/// layer answers the query alone. The same applies to an empty field list.
fn tri_query_with(tri_fields: &[Field], text: &str, match_all: bool) -> Box<dyn Query> {
    let terms: Vec<&str> = text.split_whitespace().collect();
    if tri_fields.is_empty() || terms.is_empty() {
        return Box::new(EmptyQuery);
    }
    // Under AND a gram-less term (fewer than 3 characters) would make the whole
    // conjunct false, so the trigram layer steps aside and the word layer
    // answers alone. Under OR such a term simply contributes nothing.
    if match_all && terms.iter().any(|t| trigrams(t).is_empty()) {
        return Box::new(EmptyQuery);
    }

    let term_occur = if match_all { Occur::Must } else { Occur::Should };
    let mut per_term: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for term in terms {
        let mut per_field: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for field in tri_fields {
            per_field.push((Occur::Should, tri_field_subquery(*field, term)));
        }
        per_term.push((term_occur, Box::new(BooleanQuery::new(per_field))));
    }
    Box::new(BooleanQuery::new(per_term))
}

/// Fuzzy subquery with terms AND-ed (the current default).
pub fn tri_query(tri_fields: &[Field], text: &str) -> Box<dyn Query> {
    tri_query_with(tri_fields, text, true)
}

/// Fuzzy subquery with terms OR-ed — used only for legacy saved searches that
/// were authored before the switch to AND.
pub fn tri_query_any(tri_fields: &[Field], text: &str) -> Box<dyn Query> {
    tri_query_with(tri_fields, text, false)
}

fn word_or_tri_with(
    word: Box<dyn Query>,
    tri_fields: &[Field],
    text: &str,
    match_all: bool,
) -> Box<dyn Query> {
    let tri = tri_query_with(tri_fields, text, match_all);
    Box::new(BooleanQuery::new(vec![
        (Occur::Should, Box::new(BoostQuery::new(word, 3.0))),
        (Occur::Should, Box::new(BoostQuery::new(tri, 1.0))),
    ]))
}

/// Word level (AND) OR trigram (AND).
pub fn word_or_tri(word: Box<dyn Query>, tri_fields: &[Field], text: &str) -> Box<dyn Query> {
    word_or_tri_with(word, tri_fields, text, true)
}

/// Word level (OR) OR trigram (OR) — legacy saved-search semantics.
pub fn word_or_tri_any(
    word: Box<dyn Query>,
    tri_fields: &[Field],
    text: &str,
) -> Box<dyn Query> {
    word_or_tri_with(word, tri_fields, text, false)
}

/// A `QueryParser` for free-text filters: space-separated terms are **AND**-ed,
/// matching how email clients behave (adding a word narrows the result rather
/// than widening it). Terms are still OR-ed across the supplied fields, and
/// explicit `OR`, quoted phrases and `-term` keep working.
pub fn text_parser(index: &Index, fields: Vec<Field>) -> QueryParser {
    let mut parser = QueryParser::for_index(index, fields);
    parser.set_conjunction_by_default();
    parser
}

/// The legacy `QueryParser` behaviour: space-separated terms are **OR**-ed.
pub fn any_term_parser(index: &Index, fields: Vec<Field>) -> QueryParser {
    QueryParser::for_index(index, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<String> {
        let mut tokenizer = EuroTokenizer::new();
        let mut stream = tokenizer.token_stream(text);
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        out
    }

    /// #361 core: a whole subject and a single query term must produce the same
    /// token. Under the old tokenizer the whole string was detected as Danish
    /// (stemmed) while the single word was detected as another language, so the
    /// two tokens differed — this assertion necessarily fails on the old code.
    #[test]
    fn deterministic_across_context() {
        let full = tokens("VS: Sejlerferie i Kroatien med skipper");
        let word = tokens("Sejlerferie");
        assert!(
            full.contains(&"sejlerferie".to_string()),
            "full-text tokens: {full:?}"
        );
        assert!(
            word.contains(&"sejlerferie".to_string()),
            "single-word tokens: {word:?}"
        );
    }

    /// Tokens longer than 40 characters are dropped by RemoveLongFilter.
    #[test]
    fn removes_long_tokens() {
        let long = "a".repeat(41);
        assert!(tokens(&long).is_empty());
    }

    /// Accented characters are normalized by DeunicodeFilter.
    #[test]
    fn deunicode_accents() {
        assert_eq!(tokens("Étoiles"), vec!["etoiles".to_string()]);
    }

    /// Query-side gram computation must match the tri analyzer: window →
    /// lowercase → deunicode for non-ASCII.
    #[test]
    fn trigrams_normalized_like_analyzer() {
        assert!(trigrams("ab").is_empty(), "shorter than 3 chars has no grams");
        assert_eq!(
            trigrams("Kroatiens"),
            vec!["kro", "roa", "oat", "ati", "tie", "ien", "ens"]
        );
        assert_eq!(trigrams("Été"), vec!["ete"]);
    }
}
