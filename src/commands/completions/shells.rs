use clap::ValueEnum;
use clap_complete::{Generator, Shell as CompletionShell};
use clap_complete_nushell::Nushell;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Hash)]
pub enum Shells {
    /// Bourne Again `SHell` (bash)
    Bash,
    /// Elvish shell
    Elvish,
    /// Friendly Interactive `SHell` (fish)
    Fish,
    /// `PowerShell`
    PowerShell,
    /// Z `SHell` (zsh)
    Zsh,
    /// Nushell (nu)
    Nushell,
}

impl Generator for Shells {
    fn file_name(&self, name: &str) -> String {
        match *self {
            Self::Bash => CompletionShell::Bash.file_name(name),
            Self::Elvish => CompletionShell::Elvish.file_name(name),
            Self::Fish => CompletionShell::Fish.file_name(name),
            Self::PowerShell => CompletionShell::PowerShell.file_name(name),
            Self::Zsh => CompletionShell::Zsh.file_name(name),
            Self::Nushell => Nushell.file_name(name),
        }
    }

    fn generate(&self, cmd: &clap::Command, buf: &mut dyn std::io::Write) {
        match *self {
            Self::Bash => CompletionShell::Bash.generate(cmd, buf),
            Self::Elvish => CompletionShell::Elvish.generate(cmd, buf),
            Self::Fish => CompletionShell::Fish.generate(cmd, buf),
            Self::PowerShell => CompletionShell::PowerShell.generate(cmd, buf),
            Self::Zsh => CompletionShell::Zsh.generate(cmd, buf),
            Self::Nushell => Nushell.generate(cmd, buf),
        }
    }
}

impl std::fmt::Display for Shells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
