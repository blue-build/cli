use std::io::IsTerminal;

use clap::ValueEnum;
use log::trace;
use miette::{IntoDiagnostic, Result, miette};
use serde::ser::Serialize;
use syntect::{dumps, easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum DefaultThemes {
    #[default]
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

/// Prints the file with syntax highlighting.
///
/// # Errors
/// Will error if the theme doesn't exist, the syntax doesn't exist, or the file
/// failed to serialize.
pub fn highlight(file: &str, file_type: &str, theme: Option<DefaultThemes>) -> Result<String> {
    trace!("syntax_highlighting::highlight(file, {file_type}, {theme:?})");
    if std::io::stdout().is_terminal() {
        let ss: SyntaxSet = if file_type == "dockerfile" || file_type == "Dockerfile" {
            dumps::from_uncompressed_data(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/docker_syntax.bin"
            )))
            .into_diagnostic()?
        } else {
            SyntaxSet::load_defaults_newlines()
        };
        let ts = ThemeSet::load_defaults();

        let syntax = ss
            .find_syntax_by_extension(file_type)
            .ok_or_else(|| miette!("Failed to get syntax"))?;
        let mut h = HighlightLines::new(
            syntax,
            ts.themes
                .get(theme.unwrap_or_default().to_string().as_str())
                .ok_or_else(|| miette!("Failed to get highlight theme"))?,
        );

        let mut highlighted_lines: Vec<String> = vec![];
        for line in file.lines() {
            highlighted_lines.push(syntect::util::as_24_bit_terminal_escaped(
                &h.highlight_line(line, &ss).into_diagnostic()?,
                false,
            ));
        }
        highlighted_lines.push("\x1b[0m".to_string());
        Ok(highlighted_lines.join("\n"))
    } else {
        Ok(file.to_string())
    }
}

/// Takes a serializable struct and serializes it with syntax highlighting.
///
/// # Errors
/// Will error if the theme doesn't exist, the syntax doesn't exist, or the file
/// failed to serialize.
pub fn highlight_ser<T: Serialize + std::fmt::Debug>(
    file: &T,
    file_type: &str,
    theme: Option<DefaultThemes>,
) -> Result<String> {
    trace!("syntax_highlighting::highlight_ser(file, {file_type}, {theme:?})");
    highlight(
        serde_yaml::to_string(file).into_diagnostic()?.as_str(),
        file_type,
        theme,
    )
}

/// Prints the file with syntax highlighting.
///
/// # Errors
/// Will error if the theme doesn't exist, the syntax doesn't exist, or the file
/// failed to serialize.
pub fn print(file: &str, file_type: &str, theme: Option<DefaultThemes>) -> Result<()> {
    trace!("syntax_highlighting::print(file, {file_type}, {theme:?})");
    println!("{}", highlight(file, file_type, theme)?);
    Ok(())
}

/// Takes a serializable struct and prints it out with syntax highlighting.
///
/// # Errors
/// Will error if the theme doesn't exist, the syntax doesn't exist, or the file
/// failed to serialize.
pub fn print_ser<T: Serialize + std::fmt::Debug>(
    file: &T,
    file_type: &str,
    theme: Option<DefaultThemes>,
) -> Result<()> {
    trace!("syntax_highlighting::print_ser(file, {file_type}, {theme:?})");
    print(
        serde_yaml::to_string(file).into_diagnostic()?.as_str(),
        file_type,
        theme,
    )?;
    Ok(())
}
