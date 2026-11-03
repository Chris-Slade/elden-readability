# Background

The lore in FromSoft games is notoriously difficult to understand. Sometimes that's because it's vague, but sometimes it's because it's just hard to read, giving us such cryptic lines as

> Great volumes of water serve as a bulwark guarding sleep, and an augur of the eldritch Truth.

> The broken and discarded are fully willing to cling to fleeting simulacra, earning them some modicum of sympathy.

> The fundamentalists describe the Golden Order through the powers of regression and causality. Causality is the pull between meanings; that which links all things in a chain of relation.

> The spiral is a normalized Crucible current that, one day, will form a column that stretches to the gods.

Some of the difficulty comes from the game's recondite vocabulary choices, often the result of translating certain kanji into English while preserving the archaic style of the original. Some readability metrics used here, such as Flesch-Kincaide, Coleman-Liau, Gunning fog, Lix, and SMOG, only look at word and syllable counts (or worse, letter counts), which will overindex on sentences with polysyllabic but otherwise uncomplicated word choices. The Dale-Chall readability score and Spache readability formula instead focus on the prevalence of "difficult words" and may yield a more accurate measure of crypticness.

# Setup

## Requirements

- A Rust toolchain of version 1.85 or later. Install it with [rustup](https://rustup.rs).
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
| `--dir <path>` | required | Directory containing the game text files. |

Entries are printed from most to least complex, with ties ordered by their text.
