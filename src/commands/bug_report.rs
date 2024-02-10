use crate::module_recipe::Recipe;
use crate::shadow;

use askama::Template;
use clap::Args;
use clap_complete::Shell;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use log::{debug, error, trace};
use requestty::question::{completions, Completions};
use std::borrow::Cow;
use std::time::Duration;
use typed_builder::TypedBuilder;

use super::utils::exec_cmd;
use super::BlueBuildCommand;

const UNKNOWN_SHELL: &str = "<unknown shell>";
const UNKNOWN_VERSION: &str = "<unknown version>";
const UNKNOWN_TERMINAL: &str = "<unknown terminal>";
const GITHUB_CHAR_LIMIT: usize = 8100; // Magic number accepted by Github

#[derive(Default, Debug, Clone, TypedBuilder, Args)]
pub struct BugReportRecipe {
    recipe_dir: Option<String>,
    recipe_path: Option<String>,
}

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct BugReportCommand {
    /// Path to the recipe file
    #[arg(short, long)]
    #[builder(default)]
    recipe_path: Option<String>,
}

impl BlueBuildCommand for BugReportCommand {
    fn try_run(&mut self) -> anyhow::Result<()> {
        debug!(
            "Generating bug report for hash: {}\n",
            shadow::BB_COMMIT_HASH
        );
        debug!("Shadow Versioning:\n{}", shadow::VERSION.trim());

        self.create_bugreport()
    }
}

impl BugReportCommand {
    /// Create a pre-populated GitHub issue with information about your configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if it fails to open the issue in your browser.
    /// If this happens, you can copy the generated report and open an issue manually.
    ///
    /// # Panics
    ///
    /// This function will panic if it fails to get the current shell or terminal version.
    pub fn create_bugreport(&self) -> anyhow::Result<()> {
        use colorized::{Color, Colors};

        let os_info = os_info::get();
        let recipe = self.get_recipe();

        let environment = Environment {
            os_type: os_info.os_type(),
            shell_info: get_shell_info(),
            terminal_info: get_terminal_info(),
            os_version: os_info.version().clone(),
        };

        let issue_body = match generate_github_issue(&environment, &recipe) {
            Ok(body) => body,
            Err(e) => {
                println!(
                    "{}: {e}",
                    "Failed to generate bug report".color(Colors::BrightRedFg)
                );
                return Err(e);
            }
        };

        println!(
            "\n{}\n{}\n",
            "Generated bug report:".color(Colors::BrightGreenFg),
            issue_body
                .color(Colors::BrightBlackBg)
                .color(Colors::BrightWhiteFg)
        );

        const WARNING_MESSAGE: &str = "Please copy the above report and open an issue manually.";
        let question = requestty::Question::confirm("anonymous")
            .message(
                "Forward the pre-filled report above to GitHub in your browser?"
                    .color(Colors::BrightYellowFg),
            )
            .default(true)
            .build();

        println!("{} To avoid any sensitive data from being exposed, please review the included information before proceeding.", "Warning:".color(Colors::BrightRedBg).color(Colors::BrightWhiteFg));
        println!("Data forwarded to GitHub is subject to GitHub's privacy policy. For more information, see https://docs.github.com/en/github/site-policy/github-privacy-statement.\n");
        match requestty::prompt_one(question) {
            Ok(answer) => {
                if answer.as_bool().unwrap() {
                    let link = make_github_issue_link(&issue_body);
                    if let Err(e) = open::that(&link) {
                        println!("Failed to open issue report in your browser: {e}");
                        println!("Please copy the above report and open an issue manually, or try opening the following link:\n{link}");
                        return Err(e.into());
                    }
                } else {
                    println!("{WARNING_MESSAGE}");
                }
            }
            Err(_) => {
                println!("Will not open an issue in your browser! {WARNING_MESSAGE}");
            }
        }

        println!(
            "\n{}",
            "Thanks for using the BlueBuild bug report tool!".color(Colors::BrightCyanFg)
        );

        Ok(())
    }

    fn get_recipe(&self) -> Option<Recipe> {
        let recipe_path = self.recipe_path.clone().map_or_else(
            || {
                get_config_file("recipe", "Enter path to recipe file").map_or_else(
                    |_| {
                        trace!("Failed to get recipe");
                        String::new()
                    },
                    |recipe| recipe,
                )
            },
            |recipe_path| recipe_path,
        );

        Recipe::parse(&recipe_path).ok()
    }
}

fn get_config_file(title: &str, message: &str) -> anyhow::Result<String> {
    use std::path::Path;

    let question = requestty::Question::input(title)
        .message(message)
        .auto_complete(|p, _| auto_complete(p))
        .validate(|p, _| {
            if (p.as_ref() as &Path).exists() {
                Ok(())
            } else if p.is_empty() {
                Err("No file specified. Please enter a file path".to_string())
            } else {
                Err(format!("file `{p}` doesn't exist"))
            }
        })
        .build();

    match requestty::prompt_one(question) {
        Ok(requestty::Answer::String(path)) => Ok(path),
        Ok(_) => unreachable!(),
        Err(e) => {
            trace!("Failed to get file: {}", e);
            Err(e.into())
        }
    }
}

fn auto_complete(p: String) -> Completions<String> {
    use std::path::Path;

    let current: &Path = p.as_ref();
    let (mut dir, last) = if p.ends_with('/') {
        (current, "")
    } else {
        let dir = current.parent().unwrap_or_else(|| "/".as_ref());
        let last = current
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");
        (dir, last)
    };

    if dir.to_str().unwrap().is_empty() {
        dir = ".".as_ref();
    }

    let mut files: Completions<_> = match dir.read_dir() {
        Ok(files) => files
            .flatten()
            .filter_map(|file| {
                let path = file.path();
                let is_dir = path.is_dir();
                match path.into_os_string().into_string() {
                    Ok(s) if is_dir => Some(s + "/"),
                    Ok(s) => Some(s),
                    Err(_) => None,
                }
            })
            .collect(),
        Err(_) => {
            return completions![p];
        }
    };

    if files.is_empty() {
        return completions![p];
    }

    let fuzzer = SkimMatcherV2::default();
    files.sort_by_cached_key(|file| fuzzer.fuzzy_match(file, last).unwrap_or(i64::MAX));
    files
}

// ============================================================================= //

struct Environment {
    shell_info: ShellInfo,
    os_type: os_info::Type,
    terminal_info: TerminalInfo,
    os_version: os_info::Version,
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

#[derive(Debug)]
struct ShellInfo {
    name: String,
    version: String,
}

fn get_shell_info() -> ShellInfo {
    let failure_shell_info = ShellInfo {
        name: UNKNOWN_SHELL.to_string(),
        version: UNKNOWN_VERSION.to_string(),
    };

    let current_shell = match Shell::from_env() {
        Some(shell) => shell.to_string(),
        None => return failure_shell_info,
    };

    let version = get_shell_version(&current_shell);

    ShellInfo {
        version,
        name: current_shell,
    }
}

fn get_shell_version(shell: &str) -> String {
    let time_limit = Duration::from_millis(500);
    match shell {
        "powershecll" => {
            error!("Powershell is not supported.");
            None
        }
        _ => exec_cmd(shell, &["--version"], time_limit),
    }
    .map_or_else(
        || UNKNOWN_VERSION.to_string(),
        |output| output.stdout.trim().to_string(),
    )
}

// ============================================================================= //
// Git
// ============================================================================= //

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "github_issue.j2", escape = "md")]
struct GithubIssueTemplate<'a> {
    #[builder(setter(into))]
    bb_version: Cow<'a, str>,

    #[builder(setter(into))]
    build_rust_channel: Cow<'a, str>,

    #[builder(setter(into))]
    build_time: Cow<'a, str>,

    #[builder(setter(into))]
    git_commit_hash: Cow<'a, str>,

    #[builder(setter(into))]
    os_name: Cow<'a, str>,

    #[builder(setter(into))]
    os_version: Cow<'a, str>,

    #[builder(setter(into))]
    pkg_branch_tag: Cow<'a, str>,

    #[builder(setter(into))]
    recipe: Cow<'a, str>,

    #[builder(setter(into))]
    rust_channel: Cow<'a, str>,

    #[builder(setter(into))]
    rust_version: Cow<'a, str>,

    #[builder(setter(into))]
    shell_name: Cow<'a, str>,

    #[builder(setter(into))]
    shell_version: Cow<'a, str>,

    #[builder(setter(into))]
    terminal_name: Cow<'a, str>,

    #[builder(setter(into))]
    terminal_version: Cow<'a, str>,
}

fn get_pkg_branch_tag() -> String {
    format!("{} ({})", shadow::BRANCH, shadow::LAST_TAG)
}

fn generate_github_issue(
    environment: &Environment,
    recipe: &Option<Recipe>,
) -> anyhow::Result<String> {
    let recipe = serde_yaml::to_string(recipe)?;

    let github_template = GithubIssueTemplate::builder()
        .bb_version(shadow::PKG_VERSION)
        .build_rust_channel(shadow::BUILD_RUST_CHANNEL)
        .build_time(shadow::BUILD_TIME)
        .git_commit_hash(shadow::BB_COMMIT_HASH)
        .os_name(format!("{}", environment.os_type))
        .os_version(format!("{}", environment.os_version))
        .pkg_branch_tag(get_pkg_branch_tag())
        .recipe(recipe)
        .rust_channel(shadow::RUST_CHANNEL)
        .rust_version(shadow::RUST_VERSION)
        .shell_name(environment.shell_info.name.clone())
        .shell_version(environment.shell_info.version.clone())
        .terminal_name(environment.terminal_info.name.clone())
        .terminal_version(environment.terminal_info.version.clone())
        .build();

    Ok(github_template.render()?)
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

// ============================================================================= //

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
                version: "2.3.4".to_string(),
                name: "test_shell".to_string(),
            },
            terminal_info: TerminalInfo {
                name: "test_terminal".to_string(),
                version: "5.6.7".to_string(),
            },
        };

        let recipe = Recipe::default();
        let body = generate_github_issue(&environment, &Some(recipe)).unwrap();
        let link = make_github_issue_link(&body);

        assert!(link.contains(clap::crate_version!()));
        assert!(link.contains("Linux"));
        assert!(link.contains("1.2.3"));
        assert!(link.contains("test_shell"));
        assert!(link.contains("2.3.4"));
    }
}
