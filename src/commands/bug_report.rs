use crate::shadow;
use clap_complete::Shell;
use nu_ansi_term::Style;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::utils::{exec_cmd, home_dir};

const UNKNOWN_SHELL: &str = "<unknown shell>";
const UNKNOWN_CONFIG: &str = "<unknown config>";
const UNKNOWN_VERSION: &str = "<unknown version>";
const UNKNOWN_TERMINAL: &str = "<unknown terminal>";
const GITHUB_CHAR_LIMIT: usize = 8100; // Magic number accepted by Github

struct Environment {
    shell_info: ShellInfo,
    os_type: os_info::Type,
    terminal_info: TerminalInfo,
    os_version: os_info::Version,
}

#[derive(Debug)]
struct ShellInfo {
    name: String,
    version: String,
    config: String,
}

fn get_shell_info() -> ShellInfo {
    let failure_shell_info = ShellInfo {
        name: UNKNOWN_SHELL.to_string(),
        config: UNKNOWN_CONFIG.to_string(),
        version: UNKNOWN_VERSION.to_string(),
    };

    let current_shell = match Shell::from_env() {
        Some(shell) => shell.to_string(),
        None => return failure_shell_info,
    };

    let config = get_config_path(&current_shell)
        .and_then(|config_path| fs::read_to_string(config_path).ok())
        .map_or_else(
            || UNKNOWN_CONFIG.to_string(),
            |config| config.trim().to_string(),
        );

    let version = get_shell_version(&current_shell);

    ShellInfo {
        config,
        version,
        name: current_shell.to_string(),
    }
}

pub fn create() {
    println!("{}\n", shadow::VERSION.trim());
    let os_info = os_info::get();

    let environment = Environment {
        os_type: os_info.os_type(),
        shell_info: get_shell_info(),
        terminal_info: get_terminal_info(),
        os_version: os_info.version().clone(),
    };

    let issue_body = get_github_issue_body(&environment);

    println!(
        "{}\n{issue_body}\n\n",
        Style::new().bold().paint("Generated bug report:")
    );
    println!("Forward the pre-filled report above to GitHub in your browser?");
    println!("{} To avoid any sensitive data from being exposed, please review the included information before proceeding. Data forwarded to GitHub is subject to GitHub's privacy policy.", Style::new().bold().paint("Warning:"));
    println!(
        "Enter `{}` to accept, or anything else to decline, and `{}` to confirm your choice:\n",
        Style::new().bold().paint("y"),
        Style::new().bold().paint("Enter key")
    );

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);

    if input.trim().to_lowercase() == "y" {
        let link = make_github_issue_link(&issue_body);
        if let Err(e) = open::that(&link) {
            println!("Failed to open issue report in your browser: {e}");
            println!("Please copy the above report and open an issue manually, or try opening the following link:\n{link}");
        }
    } else {
        println!("Will not open an issue in your browser! Please copy the above report and open an issue manually.");
    }
    println!("Thanks for using the BlueBuild bug report tool!");
}

fn get_pkg_branch_tag() -> &'static str {
    if !shadow::TAG.is_empty() {
        return shadow::TAG;
    }
    shadow::BRANCH
}

fn get_github_issue_body(environment: &Environment) -> String {
    let shell_syntax = match environment.shell_info.name.as_ref() {
        "powershell" | "pwsh" => "pwsh",
        "fish" => "fish",
        "cmd" => "lua",
        // GitHub does not seem to support elvish syntax highlighting.
        "elvish" => "bash",
        _ => "bash",
    };

    format!("#### Current Behavior
<!-- A clear and concise description of the behavior. -->

#### Expected Behavior
<!-- A clear and concise description of what you expected to happen. -->

#### Additional context/Screenshots
<!-- Add any other context about the problem here. If applicable, add screenshots to help explain. -->

#### Possible Solution
<!--- Only if you have suggestions on a fix for the bug -->

#### Environment
- BB version: {bb_version}
- {shell_name} version: {shell_version}
- Operating system: {os_name} {os_version}
- Terminal emulator: {terminal_name} {terminal_version}
- Git Commit Hash: {git_commit_hash}
- Branch/Tag: {pkg_branch_tag}
- Rust Version: {rust_version}
- Rust channel: {rust_channel} {build_rust_channel}
- Build Time: {build_time}

#### Relevant Shell Configuration

```{shell_syntax}
{shell_config}
```

#### BB Configuration
",
        bb_version = shadow::PKG_VERSION,
        build_rust_channel =  shadow::BUILD_RUST_CHANNEL,
        build_time =  shadow::BUILD_TIME,
        git_commit_hash =  shadow::SHORT_COMMIT,
        os_name = environment.os_type,
        os_version = environment.os_version,
        pkg_branch_tag =  get_pkg_branch_tag(),
        rust_channel =  shadow::RUST_CHANNEL,
        rust_version =  shadow::RUST_VERSION,
        shell_name = environment.shell_info.name,
        shell_config = environment.shell_info.config,
        shell_version = environment.shell_info.version,
        terminal_name = environment.terminal_info.name,
        terminal_version = environment.terminal_info.version,
    )
}

fn make_github_issue_link(body: &str) -> String {
    let escaped = urlencoding::encode(body).replace("%20", "+");

    format!(
        "https://github.com/blue-build/cli/issues/new?template={}&body={}",
        urlencoding::encode("Bug_report.md"),
        escaped
    )
    .chars()
    .take(GITHUB_CHAR_LIMIT)
    .collect()
}

#[derive(Debug)]
struct TerminalInfo {
    name: String,
    version: String,
}

fn get_terminal_info() -> TerminalInfo {
    let terminal = std::env::var("TERM_PROGRAM")
        .or_else(|_| std::env::var("LC_TERMINAL"))
        .unwrap_or_else(|_| UNKNOWN_TERMINAL.to_string());

    let version = std::env::var("TERM_PROGRAM_VERSION")
        .or_else(|_| std::env::var("LC_TERMINAL_VERSION"))
        .unwrap_or_else(|_| UNKNOWN_VERSION.to_string());

    TerminalInfo {
        name: terminal,
        version,
    }
}

fn get_config_path(shell: &str) -> Option<PathBuf> {
    if shell == "nu" {
        return dirs::config_dir().map(|config_dir| config_dir.join("nushell").join("config.nu"));
    }

    home_dir().and_then(|home_dir| {
        match shell {
            "bash" => Some(".bashrc"),
            "cmd" => Some("AppData/Local/clink/starship.lua"),
            "elvish" => Some(".elvish/rc.elv"),
            "fish" => Some(".config/fish/config.fish"),
            "ion" => Some(".config/ion/initrc"),
            "powershell" | "pwsh" => {
                if cfg!(windows) {
                    Some("Documents/PowerShell/Microsoft.PowerShell_profile.ps1")
                } else {
                    Some(".config/powershell/Microsoft.PowerShell_profile.ps1")
                }
            }
            "tcsh" => Some(".tcshrc"),
            "xonsh" => Some(".xonshrc"),
            "zsh" => Some(".zshrc"),
            _ => None,
        }
        .map(|path| home_dir.join(path))
    })
}

fn get_shell_version(shell: &str) -> String {
    let time_limit = Duration::from_millis(500);

    println!("get_shell_version({shell})");
    match shell {
        "powershell" => exec_cmd(
            shell,
            &["(Get-Host | Select Version | Format-Table -HideTableHeaders | Out-String).trim()"],
            time_limit,
        ),
        _ => exec_cmd(shell, &["--version"], time_limit),
    }
    .map_or_else(
        || UNKNOWN_VERSION.to_string(),
        |output| output.stdout.trim().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_make_github_link() {
        let environment = Environment {
            os_type: os_info::Type::Linux,
            os_version: os_info::Version::Semantic(1, 2, 3),
            shell_info: ShellInfo {
                name: "test_shell".to_string(),
                version: "2.3.4".to_string(),
                config: "No config".to_string(),
            },
            terminal_info: TerminalInfo {
                name: "test_terminal".to_string(),
                version: "5.6.7".to_string(),
            },
        };

        let body = get_github_issue_body(&environment);
        let link = make_github_issue_link(&body);

        assert!(link.contains(clap::crate_version!()));
        assert!(link.contains("Linux"));
        assert!(link.contains("1.2.3"));
        assert!(link.contains("test_shell"));
        assert!(link.contains("2.3.4"));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_get_config_path() {
        let config_path = get_config_path("bash");
        assert_eq!(home_dir().unwrap().join(".bashrc"), config_path.unwrap());
    }

    #[test]
    fn test_get_shell_info() {
        let shell_info = get_shell_info();
        assert_eq!(shell_info.name, "bash");
        assert!(shell_info.config.contains("eval"));
        println!("config.config: {}", shell_info.config);
    }
}
