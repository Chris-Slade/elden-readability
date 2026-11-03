use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use readsight::formula::round_to;
use readsight::text::TextSplitter;
use readsight::{Language, TextStatistics};

const INFLECTIONS: [(&str, &str); 15] = [
    ("s", ""),
    ("es", ""),
    ("ies", "y"),
    ("d", ""),
    ("ed", ""),
    ("ied", "y"),
    ("ing", ""),
    ("ing", "e"),
    ("r", ""),
    ("er", ""),
    ("ier", "y"),
    ("st", ""),
    ("est", ""),
    ("iest", "y"),
    ("ly", ""),
];

pub struct DaleChallList {
    familiar: HashSet<String>,
    splitter: TextSplitter,
}

impl DaleChallList {
    pub fn load(path: &Path, language: &Language) -> Result<Self, Box<dyn std::error::Error>> {
        let familiar: HashSet<String> = fs::read_to_string(path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?
            .split_whitespace()
            .map(|w| w.trim_end_matches('.').to_lowercase())
            .collect();
        if familiar.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} contains no words", path.display()),
            )
            .into());
        }
        Ok(DaleChallList {
            familiar,
            splitter: TextSplitter::new(language)?,
        })
    }

    pub fn score(&self, text: &str, stats: &TextStatistics) -> f64 {
        let words = self.splitter.split_words(text);
        if words.is_empty() {
            return 0.0;
        }
        let difficult = words.iter().filter(|w| !self.is_familiar(w)).count();
        let difficult_pct = difficult as f64 / words.len() as f64 * 100.0;
        let raw = 0.1579 * difficult_pct + 0.0496 * stats.average_words_per_sentence;
        let adjusted = if difficult_pct > 5.0 {
            raw + 3.6365
        } else {
            raw
        };
        round_to(adjusted, 1)
    }

    fn is_familiar(&self, word: &str) -> bool {
        let word = word.to_lowercase().replace('’', "'");
        let word = word.trim_matches(['\'', '-']);
        let word = word.strip_suffix("'s").unwrap_or(word);
        if word.is_empty() || self.is_familiar_form(word) {
            return true;
        }
        word.contains('-')
            && word
                .split('-')
                .filter(|part| !part.is_empty())
                .all(|part| self.is_familiar_form(part))
    }

    fn is_familiar_form(&self, word: &str) -> bool {
        self.familiar.contains(word)
            || INFLECTIONS.iter().any(|(suffix, replacement)| {
                word.strip_suffix(suffix).is_some_and(|stem| {
                    self.contains_stem(&format!("{stem}{replacement}"))
                        || undouble(stem).is_some_and(|s| self.contains_stem(s))
                })
            })
    }

    fn contains_stem(&self, stem: &str) -> bool {
        !stem.is_empty() && self.familiar.contains(stem)
    }
}

fn undouble(stem: &str) -> Option<&str> {
    let mut chars = stem.chars().rev();
    let last = chars.next()?;
    (chars.next() == Some(last) && !"aeiou".contains(last))
        .then(|| &stem[..stem.len() - last.len_utf8()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use readsight::ReadSight;

    fn list(words: &[&str]) -> DaleChallList {
        let rs = ReadSight::new("en-us").unwrap();
        DaleChallList {
            familiar: words.iter().map(|w| w.to_string()).collect(),
            splitter: TextSplitter::new(rs.language()).unwrap(),
        }
    }

    #[test]
    fn matches_inflected_forms() {
        let list = list(&["sword", "love", "happy", "stop", "big", "large"]);
        for word in [
            "Sword",
            "swords",
            "loved",
            "loving",
            "happier",
            "happiest",
            "stopped",
            "stopping",
            "bigger",
            "larger",
            "sword's",
            "sword’s",
        ] {
            assert!(list.is_familiar(word), "{word}");
        }
    }

    #[test]
    fn matches_hyphenated_words_by_parts() {
        let list = list(&["curse", "laden"]);
        assert!(list.is_familiar("Curse-laden"));
        assert!(!list.is_familiar("curse-ridden"));
    }

    #[test]
    fn rejects_unlisted_words() {
        let list = list(&["sword"]);
        assert!(!list.is_familiar("glintstone"));
        assert!(!list.is_familiar("s"));
    }
}
