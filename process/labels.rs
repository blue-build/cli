// This is needed because the cache macro interferes with clippy's ability to proper lint the
// `generate_labels` function.  It doesn't work to place this allowance on the function either,
// so we place it at the file level
#![allow(clippy::missing_errors_doc)]
use crate::drivers::opts::GetMetadataOpts;
use crate::drivers::{CiDriver, Driver, InspectDriver};
use blue_build_recipe::Recipe;
use blue_build_utils::current_timestamp;
use cached::proc_macro::cached;
use log::{trace, warn};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::path::PathBuf;

/// Function will generate the labels for an image during generation of the containerfile and
/// after the optional rechunking of an image.  It is cached to avoid recalculating the labels
/// in the case they must be re-applied after rechunking.
///
/// # Arguments
///
/// * `recipe_path`: path to a given recipe
///
/// Returns: Result<String, Report>
///
/// # Errors
///
/// Returns an error if:
/// - The recipe file cannot be parsed
/// - Unable to retrieve repository URL
/// - Unable to get metadata for the base image
/// - Unable to generate the base image reference
#[cached(result = true, key = "PathBuf", convert = r"{ recipe_path.into() }")]
pub fn generate_labels(recipe_path: &Path) -> miette::Result<String> {
    trace!("Generate LABELS for recipe: ({})", recipe_path.display());
    let recipe = Recipe::parse(recipe_path)?;

    let build_id = Driver::get_build_id().to_string();
    let source = Driver::get_repo_url()?;
    let image_metada = Driver::get_metadata(
        GetMetadataOpts::builder()
            .image(&recipe.base_image_ref()?)
            .build(),
    )?;
    let base_digest = image_metada.digest();
    let base_name = format!("{}:{}", recipe.base_image, recipe.image_version);
    let current_timestamp = current_timestamp();

    // use btree here to have nice sorting by key, makes it easier to read and analyze resulting labels
    let built_in_labels = BTreeMap::from([
        (
            blue_build_utils::constants::BUILD_ID_LABEL,
            build_id.as_str(),
        ),
        ("org.opencontainers.image.title", &recipe.name),
        ("org.opencontainers.image.description", &recipe.description),
        ("org.opencontainers.image.source", &source),
        ("org.opencontainers.image.base.digest", base_digest),
        ("org.opencontainers.image.base.name", &base_name),
        ("org.opencontainers.image.created", &current_timestamp),
    ]);

    let custom_labels = recipe.labels.unwrap_or_default();

    Ok(aggregate_labels(built_in_labels, &custom_labels))
}

fn aggregate_labels<'a>(
    mut built_in_labels: BTreeMap<&'a str, &'a str>,
    custom_labels: &'a HashMap<String, String>,
) -> String {
    if !custom_labels.contains_key("io.artifacthub.package.readme-url") {
        // adding this if not included in the custom labeling to maintain backwards compatibility since this was hardcoded into the old template
        built_in_labels.insert(
            "io.artifacthub.package.readme-url",
            "https://raw.githubusercontent.com/blue-build/cli/main/README.md",
        );
    }

    // check for any conflicting labels and warn the user
    for (k, v) in &built_in_labels {
        if custom_labels.contains_key(*k) {
            warn!("Found conflicting values for custom label form recipe: {}, custom value: {}, built-in value: {}", k, custom_labels.get(*k).unwrap(), v);
        }
    }

    built_in_labels.extend(custom_labels.iter().map(|(k, v)| (k.as_str(), v.as_str())));

    built_in_labels
        .iter()
        .map(|(k, v)| format!("LABEL {k}=\"{v}\""))
        .reduce(|a, b| format!("{a}\n{b}"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_label_generation() {
        let built_in_labels = BTreeMap::from([
            (blue_build_utils::constants::BUILD_ID_LABEL, "build_id"),
            ("org.opencontainers.image.title", "title"),
            ("org.opencontainers.image.description", "description"),
            ("org.opencontainers.image.source", "source"),
            ("org.opencontainers.image.base.digest", "digest"),
            ("org.opencontainers.image.base.name", "base_name"),
            ("org.opencontainers.image.created", "today 15:30"),
        ]);
        let custom_labels = HashMap::new();
        let labels = aggregate_labels(built_in_labels, &custom_labels);

        assert!(
            labels.contains(
                format!(
                    "LABEL {}=\"{}\"",
                    blue_build_utils::constants::BUILD_ID_LABEL,
                    "build_id"
                )
                .as_str()
            )
        );
        assert!(labels.contains("LABEL org.opencontainers.image.title=\"title\""));
        assert!(labels.contains("LABEL org.opencontainers.image.description=\"description\""));
        assert!(labels.contains("LABEL org.opencontainers.image.source=\"source\""));
        assert!(labels.contains("LABEL org.opencontainers.image.base.digest=\"digest\""));
        assert!(labels.contains("LABEL org.opencontainers.image.base.name=\"base_name\""));
        assert!(labels.contains("LABEL org.opencontainers.image.created=\"today 15:30\""));
        assert!(labels.contains("LABEL io.artifacthub.package.readme-url=\"https://raw.githubusercontent.com/blue-build/cli/main/README.md\""));
        assert!(labels.contains(blue_build_utils::constants::BUILD_ID_LABEL));
        assert_eq!(labels.split('\n').count(), 8);
    }

    #[test]
    fn test_custom_label_overwrite_generation() {
        let built_in_labels = BTreeMap::from([
            (blue_build_utils::constants::BUILD_ID_LABEL, "build_id"),
            ("org.opencontainers.image.title", "title"),
            ("org.opencontainers.image.description", "description"),
            ("org.opencontainers.image.source", "source"),
            ("org.opencontainers.image.base.digest", "digest"),
            ("org.opencontainers.image.base.name", "base_name"),
            ("org.opencontainers.image.created", "today 15:30"),
        ]);
        let custom_labels = HashMap::from([(
            "io.artifacthub.package.readme-url".to_string(),
            "https://test.html".to_string(),
        )]);
        let labels = aggregate_labels(built_in_labels, &custom_labels);

        assert!(labels.contains("LABEL io.artifacthub.package.readme-url=\"https://test.html\""));
        assert_eq!(labels.split('\n').count(), 8);
    }

    #[test]
    fn test_custom_label_addition_generation() {
        let built_in_labels = BTreeMap::from([
            (blue_build_utils::constants::BUILD_ID_LABEL, "build_id"),
            ("org.opencontainers.image.title", "title"),
            ("org.opencontainers.image.description", "description"),
            ("org.opencontainers.image.source", "source"),
            ("org.opencontainers.image.base.digest", "digest"),
            ("org.opencontainers.image.base.name", "base_name"),
            ("org.opencontainers.image.created", "today 15:30"),
        ]);
        let custom_labels =
            HashMap::from([("org.container.test".to_string(), "test1".to_string())]);
        let labels = aggregate_labels(built_in_labels, &custom_labels);

        assert!(labels.contains("LABEL org.container.test=\"test1\""));
        assert!(labels.contains(blue_build_utils::constants::BUILD_ID_LABEL));
        assert_eq!(labels.split('\n').count(), 9);
    }
}
