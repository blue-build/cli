use crate::container::Tag;
use miette::Result;
use lazy_regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaggingPolicy {
    /// Regex to match the alt-tag name (e.g., 'stable' or '.*')
    #[serde(alias = "match")]
    pub match_tag: String,

    /// List of tag templates using placeholders (e.g., '{tag}-{os_version}')
    pub tags: Vec<String>,
}

#[derive(Clone)]
pub struct TagMetadata<'a> {
    pub tag: Option<&'a str>,
    pub os_version: &'a str,
    pub timestamp: &'a str,
    pub short_sha: Option<&'a str>,
}

pub fn resolve_tag_template(template: &str, metadata: &TagMetadata) -> String {
    let mut resolved = template.to_string();

    if let Some(tag) = metadata.tag {
        resolved = resolved.replace("{tag}", tag);
    }
    resolved = resolved.replace("{os_version}", metadata.os_version);
    resolved = resolved.replace("{timestamp}", metadata.timestamp);
    if let Some(short_sha) = metadata.short_sha {
        resolved = resolved.replace("{short_sha}", short_sha);
    }

    resolved
}

pub fn apply_tagging_policies(
    alt_tags: &[Tag],
    policies: &[TaggingPolicy],
    metadata: &TagMetadata,
) -> Result<Vec<Tag>> {
    let mut expanded_tags = Vec::new();

    // Pre-compile regexes for each policy
    let compiled_policies: Vec<(Regex, &TaggingPolicy)> = policies
        .iter()
        .map(|p| {
            Regex::new(&p.match_tag)
                .map_err(|e| miette::miette!("Invalid regex in tagging policy '{}': {}", p.match_tag, e))
                .map(|re| (re, p))
        })
        .collect::<Result<Vec<_>>>()?;

    for alt in alt_tags {
        let alt_str = alt.as_str();

        // Find the first policy where the regex matches the alt-tag
        let policy = compiled_policies.iter().find(|(re, _): &&(Regex, &TaggingPolicy)| re.is_match(alt_str));

        if let Some((_, policy)) = policy {
            for template in &policy.tags {
                let mut meta = metadata.clone();
                meta.tag = Some(alt_str);
                expanded_tags.push(resolve_tag_template(template, &meta).parse()?);
            }
        }
    }

    Ok(expanded_tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_template() {
        let metadata = TagMetadata {
            tag: Some("stable"),
            os_version: "41",
            timestamp: "20241021",
            short_sha: Some("abc1234"),
        };

        assert_eq!(
            resolve_tag_template("{tag}-{os_version}", &metadata),
            "stable-41"
        );
        assert_eq!(
            resolve_tag_template("{os_version}.{timestamp}", &metadata),
            "41.20241021"
        );
        assert_eq!(
            resolve_tag_template("{tag}-{short_sha}", &metadata),
            "stable-abc1234"
        );
    }

    #[test]
    fn test_apply_policies() {
        let policies = vec![
            TaggingPolicy {
                match_tag: "stable".to_string(),
                tags: vec![
                    "{tag}".to_string(),
                    "{tag}-{os_version}".to_string(),
                    "{os_version}".to_string(),
                ],
            },
            TaggingPolicy {
                match_tag: "unstable".to_string(),
                tags: vec!["{tag}-{os_version}.{timestamp}".to_string()],
            },
        ];

        let metadata = TagMetadata {
            tag: None,
            os_version: "41",
            timestamp: "20241021",
            short_sha: None,
        };

        let alt_tags = vec!["stable".parse().unwrap(), "unstable".parse().unwrap()];

        let result = apply_tagging_policies(&alt_tags, &policies, &metadata).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].as_str(), "stable");
        assert_eq!(result[1].as_str(), "stable-41");
        assert_eq!(result[2].as_str(), "41");
        assert_eq!(result[3].as_str(), "unstable-41.20241021");
    }

    #[test]
    fn test_regex_matching() {
        let policies = vec![TaggingPolicy {
            match_tag: "^v.*$".to_string(),
            tags: vec!["release-{tag}".to_string()],
        }];

        let metadata = TagMetadata {
            tag: None,
            os_version: "41",
            timestamp: "20241021",
            short_sha: None,
        };

        let alt_tags = vec!["v1.0".parse().unwrap(), "latest".parse().unwrap()];
        let result = apply_tagging_policies(&alt_tags, &policies, &metadata).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_str(), "release-v1.0");
    }
}
