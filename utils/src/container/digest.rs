use std::{ops::Deref, str::FromStr};

use lazy_regex::regex_is_match;
use miette::bail;
use oci_client::Reference;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(String);

impl Digest {
    /// Takes the passed in `Reference` and clones it with the digest.
    #[must_use]
    pub fn reference_digest(&self, image: &Reference) -> DigestReference {
        DigestReference(image.clone_with_digest(self.0.clone()))
    }
}

impl FromStr for Digest {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digest = s.trim().to_lowercase();
        if !regex_is_match!("^(sha256:[a-f0-9]{64})$", &digest) {
            bail!("The string '{s}' is not a valid digest");
        }
        Ok(Self(digest))
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DigestReference(Reference);

impl std::fmt::Display for DigestReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for DigestReference {
    type Target = Reference;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a DigestReference> for &'a Reference {
    fn from(value: &'a DigestReference) -> Self {
        &value.0
    }
}
