use std::env;
use std::fmt::Display;
use std::io::{self, IsTerminal};

use clap::ValueEnum;

#[derive(Clone, Copy)]
pub enum Style {
    Source,
    Bold,
    Difficulty(f64),
}

impl Style {
    fn sgr(self) -> String {
        match self {
            Style::Source => "1;36".to_string(),
            Style::Bold => "1".to_string(),
            Style::Difficulty(level) => format!("38;5;{}", difficulty_color(level)),
        }
    }
}

fn difficulty_color(level: f64) -> u8 {
    let level = if level.is_nan() {
        0.5
    } else {
        level.clamp(0.0, 1.0)
    };
    let (red, green) = if level < 0.5 {
        ((level * 10.0).round() as u8, 5)
    } else {
        (5, ((1.0 - level) * 10.0).round() as u8)
    };
    16 + 36 * red + 6 * green
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ColorWhen {
    Never,
    Always,
    Auto,
}

pub struct Palette {
    enabled: bool,
}

impl Palette {
    pub fn for_stdout(when: ColorWhen) -> Self {
        let enabled = match when {
            ColorWhen::Never => false,
            ColorWhen::Always => true,
            ColorWhen::Auto => {
                let no_color = env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
                io::stdout().is_terminal() && !no_color
            }
        };
        Palette { enabled }
    }

    pub fn paint(&self, styles: &[Style], value: impl Display) -> String {
        if !self.enabled || styles.is_empty() {
            return value.to_string();
        }
        let params: Vec<String> = styles.iter().map(|s| s.sgr()).collect();
        format!("\x1b[{}m{value}\x1b[0m", params.join(";"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_runs_green_to_yellow_to_red() {
        assert_eq!(difficulty_color(0.0), 46);
        assert_eq!(difficulty_color(0.5), 226);
        assert_eq!(difficulty_color(1.0), 196);
    }

    #[test]
    fn scale_clamps_out_of_range_levels() {
        assert_eq!(difficulty_color(-3.0), 46);
        assert_eq!(difficulty_color(7.0), 196);
    }
}
