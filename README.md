# Background

The lore in FromSoft games is notoriously difficult to understand. Sometimes that's because it's vague, but sometimes it's because it's just hard to read, giving us such cryptic lines as

> Great volumes of water serve as a bulwark guarding sleep, and an augur of the eldritch Truth.

> The broken and discarded are fully willing to cling to fleeting simulacra, earning them some modicum of sympathy.

> The fundamentalists describe the Golden Order through the powers of regression and causality. Causality is the pull between meanings; that which links all things in a chain of relation.

> The spiral is a normalized Crucible current that, one day, will form a column that stretches to the gods.

Some of the difficulty comes from the game's recondite vocabulary choices, often the result of translating certain kanji into English while preserving the archaic style of the original. Some readability metrics used here, such as Flesch-Kincaide, Coleman-Liau, Gunning fog, Lix, and SMOG, only look at word and syllable counts (or worse, letter counts), which will overindex on sentences with polysyllabic but otherwise uncomplicated word choices. The Dale-Chall readability score[^1] and Spache readability formula instead focus on the prevalence of "difficult words" and may yield a more accurate measure of crypticness.

[^1]: `readsight`'s implementation uses a simplified approach that treats every word with one syllable as difficult, rather than using the original list of 3,000 words proposed by Dale and Chall. Because of that it might actually be worse than the other metrics. To get the proper score, pass the `--dale-chall-list` to specify a word list to use.

# Setup

## Requirements

- A Rust toolchain of version 1.88 or later. Install it with [rustup](https://rustup.rs).
- The game text in the original XML format, in a directory passed with `--dir`.

The program reads the `*Caption*.fmg.xml` files for item descriptions and `TalkMsg.fmg.xml` for NPC dialogue.

## Usage

```sh
cargo run --release -- --dir <path> [OPTIONS]
```

| Option | Default | Description |
|---|---|---|
| `--sort <formula>` | `flesch-kincaid-grade-level` | Formula to rank by. One of `flesch-reading-ease`, `flesch-kincaid-grade-level`, `gunning-fog`, `smog`, `coleman-liau`, `ari`, `lix`, `dale-chall`, `spache`. |
| `--limit <n>` | none | Maximum number of entries to print. |
| `--min-words <n>` | `0` | Skip entries with fewer than `n` words. |
| `--dale-chall-list <file>` | none | Word list of familiar words for the Dale-Chall score, separated by whitespace. Inflected forms of listed words (plurals, possessives, `-ed`, `-ing`, `-er`, `-est`, `-ly`) and hyphenated compounds of listed words also count as familiar. Without this option, readsight's approximation is used, which counts every word with more than one syllable as difficult. |
| `--dir <path>` | required | Directory containing the game text files. |

Entries are printed from most to least complex, with ties ordered by their text.

Near-duplicate entries are merged before scoring, using MinHash signatures over 3-word shingles with an LSH index from [`txtfp`](https://crates.io/crates/txtfp). Entries with an estimated Jaccard similarity of 0.9 or more to an earlier entry are merged into it. This collapses weapon captions repeated for each affinity, upgrade levels of Spirit Ashes, and dialogue lines that differ only in punctuation or capitalization. Only the first entry in each group is scored and shown, and the IDs of the merged entries are listed on its `duplicates:` line.
