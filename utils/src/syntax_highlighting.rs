use anyhow::{anyhow, Result};
use clap::ValueEnum;
use serde::ser::Serialize;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

const DOCKERFILE_SYNTAX: &str = include_str!("../highlights/Dockerfile.sublime-syntax");

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DefaultThemes {
    MochaDark,
    OceanDark,
    OceanLight,
    EightiesDark,
    InspiredGithub,
    SolarizedDark,
    SolarizedLight,
}

impl std::fmt::Display for DefaultThemes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::MochaDark => "base16-mocha.dark",
            Self::OceanDark => "base16-ocean.dark",
            Self::OceanLight => "base16-ocean.light",
            Self::EightiesDark => "base16-eighties.dark",
            Self::InspiredGithub => "InspiredGithub",
            Self::SolarizedDark => "Solarized (dark)",
            Self::SolarizedLight => "Solarized (light)",
        })
    }
}

pub fn print(file: &str, file_type: &str, theme: Option<DefaultThemes>) -> Result<()> {
    let ss = SyntaxSet::load_defaults_newlines();
    for synt in ss.syntaxes() {
        println!("Name: {}, Ext: {:?}", synt.name, synt.file_extensions);
    }
    let ts = ThemeSet::load_defaults();

    let syntax = ss
        .find_syntax_by_extension(file_type)
        .ok_or_else(|| anyhow!("Failed to get syntax"))?;
    let mut h = HighlightLines::new(
        syntax,
        ts.themes
            .get(
                theme
                    .map_or_else(|| "base16-mocha.dark".to_string(), |t| t.to_string())
                    .as_str(),
            )
            .ok_or_else(|| anyhow!("Failed to get highlight theme"))?,
    );
    for line in file.lines() {
        let ranges = h.highlight_line(line, &ss)?;
        let escaped = syntect::util::as_24_bit_terminal_escaped(&ranges, false);
        println!("{escaped}");
    }
    println!("\x1b[0m");
    Ok(())
}

/// Takes a serializable struct and prints it out with syntax highlighting.
///
/// # Errors
/// Will error if the theme doesn't exist, the syntax doesn't exist, or the file
/// failed to serialize.
pub fn print_ser<T: Serialize>(
    file: &T,
    file_type: &str,
    theme: Option<DefaultThemes>,
) -> Result<()> {
    print(serde_yaml::to_string(file)?.as_str(), file_type, theme)?;
    Ok(())
}
