use jsonschema::paths::{LazyLocation, Location as JsonLocation};
use serde::Deserialize;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location(JsonLocation);

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for Location {
    type Target = JsonLocation;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Location {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::hash::Hash for Location {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl From<JsonLocation> for Location {
    fn from(value: JsonLocation) -> Self {
        Self(value)
    }
}

impl From<&JsonLocation> for Location {
    fn from(value: &JsonLocation) -> Self {
        Self(value.clone())
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<&str> for Location {
    type Error = miette::Report;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        fn child<'a, 'b, 'c, I>(path_iter: &mut I, location: &'b LazyLocation<'b, 'a>) -> Location
        where
            I: Iterator<Item = &'c str>,
        {
            let Some(path) = path_iter.next() else {
                return Location(JsonLocation::from(location));
            };
            let location = build(path, location);
            child(path_iter, &location)
        }

        fn build<'a, 'b>(
            path: &'a str,
            location: &'b LazyLocation<'b, 'a>,
        ) -> LazyLocation<'a, 'b> {
            path.parse::<usize>()
                .map_or_else(|_| location.push(path), |p| location.push(p))
        }
        let path_count = value.split('/').count();
        let mut path_iter = value.split('/');

        let root = path_iter.next().unwrap();

        if root.is_empty() && path_count == 1 {
            return Ok(Self::default());
        }

        let Some(path) = path_iter.next() else {
            return Ok(Self(JsonLocation::from(&LazyLocation::new())));
        };

        let location = LazyLocation::new();
        let location = build(path, &location);

        Ok(child(&mut path_iter, &location))
    }
}

impl TryFrom<&String> for Location {
    type Error = miette::Report;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<String> for Location {
    type Error = miette::Report;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}
