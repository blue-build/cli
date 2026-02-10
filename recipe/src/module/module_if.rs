use std::{collections::BTreeMap, path::PathBuf};

use blue_build_utils::or_list::OrList;
use bon::Builder;
use cached::proc_macro::cached;
use comlexr::cmd;
use log::{debug, trace};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use blue_build_utils::test_utils::{get_env_var, has_env_var};

#[cfg(not(test))]
use blue_build_utils::{get_env_var, has_env_var};

#[derive(Debug, Clone, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum ModuleIf {
    Eval(String),
    Complex(Box<ModuleIfComplex>),
}

impl ModuleIf {
    #[must_use]
    pub fn should_template(&self) -> bool {
        #[cached(convert = r#"{ mod_if.to_owned() }"#, key = "ModuleIf")]
        fn inner(mod_if: &ModuleIf) -> bool {
            match mod_if {
                ModuleIf::Eval(_) => true,
                ModuleIf::Complex(if_clause) => {
                    let ModuleIfComplex {
                        // template time checks
                        host_file,
                        host_env,
                        host_exec,
                        not_host_exec,

                        // runtime checks
                        os_release: _,
                        not_os_release: _,
                        env: _,
                        eval: _,
                    } = if_clause.as_ref();

                    let host_file = host_file
                        .as_ref()
                        .is_none_or(ModuleIfHostFile::should_template);
                    let host_env = host_env.as_ref().is_none_or(ModuleIfEnv::should_template);
                    let host_exec = host_exec.as_ref().is_none_or(|host_exec| {
                        {
                            let c = cmd!(&host_exec.command, for &host_exec.args);
                            debug!("Running check: {c:?}");
                            c
                        }
                        .output()
                        .inspect(|output| {
                            trace!("stdout:{}", String::from_utf8_lossy(&output.stdout));
                            trace!("stderr:{}", String::from_utf8_lossy(&output.stderr));
                        })
                        .is_ok_and(|out| out.status.success())
                    });
                    let not_host_exec = not_host_exec.as_ref().is_none_or(|not_host_exec| {
                        {
                            let c = cmd!(&not_host_exec.command, for &not_host_exec.args);
                            debug!("Running negated check: {c:?}");
                            c
                        }
                        .output()
                        .inspect(|output| {
                            trace!("stdout:{}", String::from_utf8_lossy(&output.stdout));
                            trace!("stderr:{}", String::from_utf8_lossy(&output.stderr));
                        })
                        .map_or(true, |out| !out.status.success())
                    });

                    host_file && host_env && host_exec && not_host_exec
                }
            }
        }
        inner(self)
    }
}

#[derive(Debug, Clone, Builder, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleIfComplex {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_release: Option<BTreeMap<String, OrList<String>>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_os_release: Option<BTreeMap<String, OrList<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<ModuleIfEnv>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_file: Option<ModuleIfHostFile>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_env: Option<ModuleIfEnv>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_exec: Option<ModuleIfHostExec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_host_exec: Option<ModuleIfHostExec>,
}

#[derive(Debug, Clone, Builder, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleIfEnv {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<OrList<String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_exists: Option<OrList<String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals: Option<BTreeMap<String, OrList<String>>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_equals: Option<BTreeMap<String, OrList<String>>>,
}

impl ModuleIfEnv {
    fn should_template(&self) -> bool {
        let exists = self.exists.as_ref().is_none_or(|exists| match exists {
            OrList::Single(var) => has_env_var(var),
            OrList::List(list) => list.iter().all(|var| has_env_var(var)),
        });
        let not_exists = self
            .not_exists
            .as_ref()
            .is_none_or(|not_exists| match not_exists {
                OrList::Single(var) => !has_env_var(var),
                OrList::List(list) => list.iter().all(|var| !has_env_var(var)),
            });
        let equals = self.equals.as_ref().is_none_or(|equals| {
            equals.iter().all(|(key, value)| match value {
                OrList::Single(value) => get_env_var(key).is_ok_and(|v| *value == v),
                OrList::List(list) => list
                    .iter()
                    .any(|value| get_env_var(key).is_ok_and(|v| *value == v)),
            })
        });
        let not_equals = self.not_equals.as_ref().is_none_or(|not_equals| {
            not_equals.iter().all(|(key, value)| match value {
                OrList::Single(value) => get_env_var(key).map_or(true, |v| *value != v),
                OrList::List(list) => list
                    .iter()
                    .all(|value| get_env_var(key).map_or(true, |v| *value != v)),
            })
        });

        exists && not_exists && equals && not_equals
    }
}

#[derive(Debug, Clone, Builder, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleIfHostFile {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<OrList<PathBuf>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_exists: Option<OrList<PathBuf>>,
}

impl ModuleIfHostFile {
    fn should_template(&self) -> bool {
        let exists = self.exists.as_ref().is_none_or(|exists| match exists {
            OrList::Single(path) => {
                debug!("Checking if {} exists", path.display());
                path.exists()
            }
            OrList::List(list) => list.iter().all(|path| {
                debug!("Checking if {} exists", path.display());
                path.exists()
            }),
        });
        let not_exists = self
            .not_exists
            .as_ref()
            .is_none_or(|not_exists| match not_exists {
                OrList::Single(path) => !path.exists(),
                OrList::List(list) => list.iter().all(|path| !path.exists()),
            });

        exists && not_exists
    }
}

#[derive(Debug, Clone, Builder, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleIfHostExec {
    pub command: String,

    #[serde(default)]
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::<String>::is_empty")]
    pub args: Vec<String>,
}

#[cfg(test)]
mod test {
    use blue_build_utils::test_utils::set_env_var;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::single_exists_pass(
        ModuleIfHostFile::builder()
            .exists("Cargo.toml")
            .build(),
        true
    )]
    #[case::multi_exists_pass(
        ModuleIfHostFile::builder()
            .exists(vec![
                "Cargo.toml",
                "src/recipe.rs",
            ])
            .build(),
        true
    )]
    #[case::single_exists_fail(
        ModuleIfHostFile::builder()
            .exists("Cargo.lock")
            .build(),
        false
    )]
    #[case::multi_exists_fail(
        ModuleIfHostFile::builder()
            .exists(vec![
                "Cargo.lock",
                "utils/",
            ])
            .build(),
        false
    )]
    #[case::single_not_exists_fail(
        ModuleIfHostFile::builder()
            .not_exists("Cargo.toml")
            .build(),
        false
    )]
    #[case::multi_not_exists_fail(
        ModuleIfHostFile::builder()
            .not_exists(vec![
                "Cargo.toml",
                "src/recipe.rs",
            ])
            .build(),
        false
    )]
    #[case::single_not_exists_pass(
        ModuleIfHostFile::builder()
            .not_exists("Cargo.lock")
            .build(),
        true
    )]
    #[case::multi_not_exists_pass(
        ModuleIfHostFile::builder()
            .not_exists(vec![
                "Cargo.lock",
                "utils/",
            ])
            .build(),
        true
    )]
    #[case::both_pass(
        ModuleIfHostFile::builder()
            .exists("Cargo.toml")
            .not_exists(vec![
                "Cargo.lock",
                "utils/",
            ])
            .build(),
        true
    )]
    #[case::both_fail(
        ModuleIfHostFile::builder()
            .exists(vec![
                "Cargo.lock",
                "utils/",
            ])
            .not_exists("Cargo.toml")
            .build(),
        false
    )]
    fn if_host_file(#[case] host_file: ModuleIfHostFile, #[case] expected: bool) {
        assert_eq!(host_file.should_template(), expected);
    }

    #[rstest]
    #[case::exists_single_pass(
        &[
            (
                "TEST".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .exists("TEST")
            .build(),
        true
    )]
    #[case::exists_single_fail(
        &[],
        ModuleIfEnv::builder()
            .exists("TEST")
            .build(),
        false
    )]
    #[case::exists_multi_pass(
        &[
            (
                "TEST".into(),
                "val".into(),
            ),
            (
                "TEST_2".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .exists(vec![
                "TEST",
                "TEST_2"
            ])
            .build(),
        true
    )]
    #[case::exists_multi_fail(
        &[],
        ModuleIfEnv::builder()
            .exists(vec![
                "TEST",
                "TEST_2"
            ])
            .build(),
        false
    )]
    #[case::not_exists_single_pass(
        &[],
        ModuleIfEnv::builder()
            .not_exists("TEST")
            .build(),
        true
    )]
    #[case::not_exists_single_fail(
        &[
            (
                "TEST".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .not_exists("TEST")
            .build(),
        false
    )]
    #[case::not_exists_multi_pass(
        &[],
        ModuleIfEnv::builder()
            .not_exists(vec![
                "TEST",
                "TEST_2"
            ])
            .build(),
        true
    )]
    #[case::not_exists_multi_fail(
        &[
            (
                "TEST".into(),
                "val".into(),
            ),
        ],
        ModuleIfEnv::builder()
            .not_exists(vec![
                "TEST",
                "TEST_2"
            ])
            .build(),
        false
    )]
    #[case::equals_single_pass(
        &[
            (
                "TEST".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .equals([("TEST".into(), "val".into())])
            .build(),
        true
    )]
    #[case::equals_single_fail(
        &[],
        ModuleIfEnv::builder()
            .equals([("TEST".into(), "val".into())])
            .build(),
        false
    )]
    #[case::equals_multi_pass(
        &[
            (
                "TEST".into(),
                "val".into(),
            ),
            (
                "TEST_2".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .equals([("TEST".into(), vec!["val", "test"].into())])
            .build(),
        true
    )]
    #[case::equals_multi_fail(
        &[],
        ModuleIfEnv::builder()
            .equals([("TEST".into(), vec!["val", "test"].into())])
            .build(),
        false
    )]
    #[case::not_equals_single_fail(
        &[
            (
                "TEST".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .not_equals([("TEST".into(), "val".into())])
            .build(),
        false
    )]
    #[case::not_equals_single_pass(
        &[],
        ModuleIfEnv::builder()
            .not_equals([("TEST".into(), "val".into())])
            .build(),
        true
    )]
    #[case::not_equals_multi_fail(
        &[
            (
                "TEST".into(),
                "val".into(),
            ),
            (
                "TEST_2".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .not_equals([("TEST".into(), vec!["val", "test"].into())])
            .build(),
        false
    )]
    #[case::not_equals_multi_pass(
        &[],
        ModuleIfEnv::builder()
            .not_equals([("TEST".into(), vec!["val", "test"].into())])
            .build(),
        true
    )]
    #[case::all_pass(
        &[
            (
                "TEST".into(),
                "val".into(),
            ),
            (
                "TEST_2".into(),
                "val".into(),
            )
        ],
        ModuleIfEnv::builder()
            .exists(vec![
                "TEST",
                "TEST_2"
            ])
            .not_exists("TEST_3")
            .equals([("TEST".into(), "val".into())])
            .not_equals([("TEST_2".into(), vec!["test1", "test"].into())])
            .build(),
        true
    )]
    #[case::all_fail(
        &[
            (
                "TEST_3".into(),
                "test1".into(),
            )
        ],
        ModuleIfEnv::builder()
            .exists(vec![
                "TEST",
                "TEST_2"
            ])
            .not_exists("TEST_3")
            .equals([("TEST".into(), "val".into())])
            .not_equals([("TEST_3".into(), vec!["test1", "test"].into())])
            .build(),
        false
    )]
    fn if_env(
        #[case] set_vars: &[(String, String)],
        #[case] host_env: ModuleIfEnv,
        #[case] expected: bool,
    ) {
        for (key, value) in set_vars {
            set_env_var(key, value);
        }

        assert_eq!(host_env.should_template(), expected);
    }
}
