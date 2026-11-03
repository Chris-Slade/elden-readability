mod dale_chall;
mod dedupe;
mod style;

use std::error::Error;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use readsight::{ReadSight, TextStatistics};

use dale_chall::DaleChallList;
use dedupe::{Group, dedupe};
use style::{ColorWhen, Palette, Style};

const PLACEHOLDERS: [&str; 2] = ["%null%", "(dummyText)"];
const SENTENCE_ENDINGS: [char; 3] = ['.', '!', '?'];
const CLOSING_CHARS: [char; 7] = ['"', '\'', '”', '’', ')', ']', '>'];

#[derive(Parser)]
#[command(about = "Rank Elden Ring game text by readability complexity")]
struct Args {
    #[arg(long, value_enum, default_value_t = Formula::FleschKincaidGradeLevel)]
    sort: Formula,

    #[arg(long)]
    limit: Option<usize>,

    #[arg(long, default_value_t = 0)]
    min_words: i64,

    #[arg(long, value_name = "FILE")]
    dale_chall_list: Option<PathBuf>,

    #[arg(
        long,
        value_enum,
        value_name = "WHEN",
        num_args = 0..=1,
        default_value_t = ColorWhen::Auto,
        default_missing_value = "auto"
    )]
    color: ColorWhen,

    #[arg(long)]
    dir: PathBuf,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Formula {
    FleschReadingEase,
    FleschKincaidGradeLevel,
    GunningFog,
    Smog,
    ColemanLiau,
    Ari,
    Lix,
    DaleChall,
    Spache,
}

impl Formula {
    fn id(self) -> &'static str {
        match self {
            Formula::FleschReadingEase => "flesch_reading_ease",
            Formula::FleschKincaidGradeLevel => "flesch_kincaid_grade_level",
            Formula::GunningFog => "gunning_fog",
            Formula::Smog => "smog",
            Formula::ColemanLiau => "coleman_liau",
            Formula::Ari => "ari",
            Formula::Lix => "lix",
            Formula::DaleChall => "dale_chall",
            Formula::Spache => "spache",
        }
    }

    fn easy_and_hard_scores(self) -> (f64, f64) {
        match self {
            Formula::FleschReadingEase => (90.0, 30.0),
            Formula::FleschKincaidGradeLevel
            | Formula::Smog
            | Formula::ColemanLiau
            | Formula::Ari => (6.0, 16.0),
            Formula::GunningFog => (6.0, 17.0),
            Formula::Lix => (25.0, 60.0),
            Formula::DaleChall => (4.9, 9.9),
            Formula::Spache => (1.0, 4.0),
        }
    }

    fn higher_is_harder(self) -> bool {
        let (easy, hard) = self.easy_and_hard_scores();
        hard > easy
    }

    fn difficulty(self, score: f64) -> f64 {
        let (easy, hard) = self.easy_and_hard_scores();
        (score - easy) / (hard - easy)
    }
}

struct Entry {
    source: String,
    id: u32,
    text: String,
}

struct Scores {
    flesch_reading_ease: f64,
    flesch_kincaid_grade_level: f64,
    gunning_fog: f64,
    smog: f64,
    coleman_liau: f64,
    ari: f64,
    lix: f64,
    dale_chall: f64,
    spache: f64,
}

impl Scores {
    fn compute(
        rs: &ReadSight,
        text: &str,
        stats: &TextStatistics,
        dale_chall_list: Option<&DaleChallList>,
    ) -> readsight::Result<Self> {
        let calc = |f: Formula| {
            rs.registry()
                .calculate(f.id(), rs.language(), stats)
                .map(|r| r.score)
        };
        Ok(Scores {
            flesch_reading_ease: calc(Formula::FleschReadingEase)?,
            flesch_kincaid_grade_level: calc(Formula::FleschKincaidGradeLevel)?,
            gunning_fog: calc(Formula::GunningFog)?,
            smog: calc(Formula::Smog)?,
            coleman_liau: calc(Formula::ColemanLiau)?,
            ari: calc(Formula::Ari)?,
            lix: calc(Formula::Lix)?,
            dale_chall: match dale_chall_list {
                Some(list) => list.score(text, stats),
                None => calc(Formula::DaleChall)?,
            },
            spache: calc(Formula::Spache)?,
        })
    }

    fn get(&self, formula: Formula) -> f64 {
        match formula {
            Formula::FleschReadingEase => self.flesch_reading_ease,
            Formula::FleschKincaidGradeLevel => self.flesch_kincaid_grade_level,
            Formula::GunningFog => self.gunning_fog,
            Formula::Smog => self.smog,
            Formula::ColemanLiau => self.coleman_liau,
            Formula::Ari => self.ari,
            Formula::Lix => self.lix,
            Formula::DaleChall => self.dale_chall,
            Formula::Spache => self.spache,
        }
    }
}

struct ScoredEntry {
    entry: Entry,
    duplicates: Vec<Entry>,
    scores: Scores,
}

fn source_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                (n.contains("Caption") && n.ends_with(".xml")) || n == "TalkMsg.fmg.xml"
            })
        })
        .collect();
    files.sort();
    Ok(files)
}

fn parse_fmg(path: &Path) -> Result<Vec<Entry>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let source = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.split('.').next())
        .unwrap_or_default()
        .to_string();

    let mut reader = Reader::from_str(content.trim_start_matches('\u{feff}'));
    let mut entries = Vec::new();
    let mut current: Option<(u32, String)> = None;

    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == "text" => {
                let id = e
                    .try_get_attribute("id")?
                    .ok_or("text element without id")?;
                let id = id.value.parse()?;
                current = Some((id, String::new()));
            }
            Event::Text(t) => {
                if let Some((_, buf)) = current.as_mut() {
                    buf.push_str(&t);
                }
            }
            Event::CData(t) => {
                if let Some((_, buf)) = current.as_mut() {
                    buf.push_str(&t);
                }
            }
            Event::GeneralRef(r) => {
                if let Some((_, buf)) = current.as_mut() {
                    match r.resolve_char_ref()? {
                        Some(c) => buf.push(c),
                        None => {
                            let name: &str = &r;
                            buf.push_str(
                                resolve_predefined_entity(name)
                                    .ok_or_else(|| format!("unknown entity &{name};"))?,
                            );
                        }
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == "text" => {
                if let Some((id, buf)) = current.take() {
                    let text = buf.trim();
                    if !text.is_empty() && !PLACEHOLDERS.contains(&text) {
                        entries.push(Entry {
                            source: source.clone(),
                            id,
                            text: text.to_string(),
                        });
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(entries)
}

fn split_lines_into_sentences(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.trim();
            let line = line.strip_prefix("- ").unwrap_or(line);
            let terminated = line
                .trim_end_matches(CLOSING_CHARS)
                .ends_with(SENTENCE_ENDINGS);
            if line.is_empty() || terminated {
                line.to_string()
            } else {
                format!("{line}.")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn score(
    rs: &ReadSight,
    group: Group<Entry>,
    min_words: i64,
    dale_chall_list: Option<&DaleChallList>,
) -> readsight::Result<Option<ScoredEntry>> {
    let Group {
        representative: entry,
        duplicates,
    } = group;
    let text = split_lines_into_sentences(&entry.text);
    let stats = match rs.analyze(&text) {
        Ok(stats) if stats.word_count >= min_words => stats,
        Ok(_) | Err(readsight::Error::EmptyText) => return Ok(None),
        Err(e) => return Err(e),
    };
    let scores = Scores::compute(rs, &text, &stats, dale_chall_list)?;
    Ok(Some(ScoredEntry {
        entry,
        duplicates,
        scores,
    }))
}

fn write_entry(
    out: &mut impl Write,
    scored: &ScoredEntry,
    sort: Formula,
    palette: &Palette,
) -> io::Result<()> {
    let ScoredEntry {
        entry,
        duplicates,
        scores,
    } = scored;
    let format_score = |formula: Formula, emphasis: &[Style]| {
        let score = scores.get(formula);
        let difficulty = [Style::Difficulty(formula.difficulty(score))];
        format!(
            "{}={}",
            palette.paint(emphasis, formula.id()),
            palette.paint(&[emphasis, &difficulty].concat(), score)
        )
    };
    writeln!(
        out,
        "{} {}",
        palette.paint(
            &[Style::Source],
            format!("[{} #{}]", entry.source, entry.id)
        ),
        format_score(sort, &[Style::Bold])
    )?;
    let all: Vec<String> = Formula::value_variants()
        .iter()
        .map(|&f| format_score(f, if f == sort { &[Style::Bold] } else { &[] }))
        .collect();
    writeln!(out, "  {}", all.join(" "))?;
    if !duplicates.is_empty() {
        let ids: Vec<String> = duplicates
            .iter()
            .map(|d| format!("{} #{}", d.source, d.id))
            .collect();
        writeln!(out, "  duplicates: {}", ids.join(", "))?;
    }
    for line in entry.text.lines() {
        writeln!(out, "    {line}")?;
    }
    writeln!(out)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let rs = ReadSight::new("en-us")?;
    let dale_chall_list = args
        .dale_chall_list
        .as_deref()
        .map(|path| DaleChallList::load(path, rs.language()))
        .transpose()?;

    let mut entries = Vec::new();
    for path in source_files(&args.dir)? {
        entries.extend(parse_fmg(&path)?);
    }

    let mut results = Vec::new();
    for group in dedupe(entries, |e| &e.text)? {
        if let Some(scored) = score(&rs, group, args.min_words, dale_chall_list.as_ref())? {
            results.push(scored);
        }
    }

    let sort = args.sort;
    results.sort_by(|a, b| {
        let ordering = a.scores.get(sort).total_cmp(&b.scores.get(sort));
        let ordering = if sort.higher_is_harder() {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| a.entry.text.cmp(&b.entry.text))
    });

    let palette = Palette::for_stdout(args.color);
    let shown = &results[..results.len().min(args.limit.unwrap_or(usize::MAX))];
    match write_results(shown, sort, &palette) {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => Ok(result?),
    }
}

fn write_results(results: &[ScoredEntry], sort: Formula, palette: &Palette) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    for scored in results {
        write_entry(&mut out, scored, sort, palette)?;
    }
    out.flush()
}
