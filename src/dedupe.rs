use txtfp::{
    Canonicalizer, Error, Fingerprinter, LshIndex, LshIndexBuilder, MinHashFingerprinter,
    MinHashSig, ShingleTokenizer, WordTokenizer, jaccard,
};

const SIGNATURE_SIZE: usize = 128;
const SHINGLE_WORDS: usize = 3;
const SIMILARITY_THRESHOLD: f32 = 0.9;

pub struct Group<T> {
    pub representative: T,
    pub duplicates: Vec<T>,
}

pub fn dedupe<T>(items: Vec<T>, text: impl Fn(&T) -> &str) -> txtfp::Result<Vec<Group<T>>> {
    let fingerprinter = MinHashFingerprinter::<_, SIGNATURE_SIZE>::new(
        Canonicalizer::default(),
        ShingleTokenizer {
            k: SHINGLE_WORDS,
            inner: WordTokenizer,
        },
    );
    let mut index: LshIndex<SIGNATURE_SIZE> =
        LshIndexBuilder::for_threshold(SIMILARITY_THRESHOLD, SIGNATURE_SIZE)?.build();
    let mut groups: Vec<Group<T>> = Vec::new();

    for item in items {
        let signature = match fingerprinter.fingerprint(text(&item)) {
            Ok(signature) => signature,
            Err(Error::InvalidInput(_)) => {
                groups.push(Group {
                    representative: item,
                    duplicates: Vec::new(),
                });
                continue;
            }
            Err(e) => return Err(e),
        };
        match closest_match(&index, &signature) {
            Some(group) => groups[group].duplicates.push(item),
            None => {
                index.insert(groups.len() as u64, signature);
                groups.push(Group {
                    representative: item,
                    duplicates: Vec::new(),
                });
            }
        }
    }
    Ok(groups)
}

fn closest_match(
    index: &LshIndex<SIGNATURE_SIZE>,
    signature: &MinHashSig<SIGNATURE_SIZE>,
) -> Option<usize> {
    index
        .query_with_threshold(signature, SIMILARITY_THRESHOLD)
        .into_iter()
        .filter_map(|id| index.get(id).map(|other| (id, jaccard(signature, other))))
        .max_by(|(a_id, a), (b_id, b)| a.total_cmp(b).then(b_id.cmp(a_id)))
        .map(|(id, _)| id as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_texts(texts: &[&'static str]) -> Vec<(&'static str, Vec<&'static str>)> {
        dedupe(texts.to_vec(), |t| t)
            .unwrap()
            .into_iter()
            .map(|g| (g.representative, g.duplicates))
            .collect()
    }

    #[test]
    fn merges_exact_duplicates_into_first_occurrence() {
        let groups = group_texts(&[
            "Curved greatsword wielded by Bloodhound Knights.",
            "A fearsome blade capable of brutal airborne attacks.",
            "Curved greatsword wielded by Bloodhound Knights.",
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups[0].1,
            vec!["Curved greatsword wielded by Bloodhound Knights."]
        );
        assert!(groups[1].1.is_empty());
    }

    #[test]
    fn keeps_distinct_texts_separate() {
        let groups = group_texts(&["Ngh!", "Hmm.", "The Tarnished will soon return."]);
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn keeps_texts_without_words() {
        let groups = group_texts(&["...", "..."]);
        assert_eq!(groups.len(), 2);
    }
}
