//! Inline glosses: `word[≤3 word explanation]`, e.g. `akṣara[letter; imperishable]`.

pub const MAX_WORDS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gloss<'a> {
    pub word: &'a str,
    pub explanation: &'a str,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GlossError<'a> {
    #[error("gloss `[{0}]` is not attached to a word")]
    Detached(&'a str),
    #[error("gloss `{word}[{explanation}]` has {words} words (max {MAX_WORDS})")]
    TooLong { word: &'a str, explanation: &'a str, words: usize },
    #[error("unclosed `[` after `{0}`")]
    Unclosed(&'a str),
}

/// Every bracketed gloss in `text`, in order.
pub fn scan(text: &str) -> impl Iterator<Item = Result<Gloss<'_>, GlossError<'_>>> {
    let mut rest = text;
    let mut consumed = 0;
    std::iter::from_fn(move || {
        let open = rest.find('[')?;
        let word = text[..consumed + open].rsplit(char::is_whitespace).next().unwrap_or_default();
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            rest = "";
            return Some(Err(GlossError::Unclosed(word)));
        };
        let explanation = &after[..close];
        consumed += open + 1 + close + 1;
        rest = &after[close + 1..];
        let words = explanation.split_whitespace().count();
        Some(if word.is_empty() {
            Err(GlossError::Detached(explanation))
        } else if words > MAX_WORDS {
            Err(GlossError::TooLong { word, explanation, words })
        } else {
            Ok(Gloss { word, explanation })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_glosses() {
        let found: Vec<_> = scan("the akṣara[letter; imperishable] and logos[word, reason].").collect();
        assert_eq!(
            found,
            vec![
                Ok(Gloss { word: "akṣara", explanation: "letter; imperishable" }),
                Ok(Gloss { word: "logos", explanation: "word, reason" }),
            ]
        );
    }

    #[test]
    fn rejects_bad_glosses() {
        assert!(matches!(scan("a [loose note]").next(), Some(Err(GlossError::Detached(_)))));
        assert!(matches!(scan("x[one two three four]").next(), Some(Err(GlossError::TooLong { words: 4, .. }))));
    }
}
