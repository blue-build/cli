use std::sync::Arc;

use jsonschema::paths::{LazyLocation, Location as JsonLocation, LocationSegment};

#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct Location(Arc<String>);

impl Location {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&JsonLocation> for Location {
    fn from(value: &JsonLocation) -> Self {
        Self(Arc::new(value.as_str().into()))
    }
}

impl From<JsonLocation> for Location {
    fn from(value: JsonLocation) -> Self {
        Self(Arc::new(value.as_str().into()))
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
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
                return JsonLocation::from(location).into();
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
            return Ok(Self::from(JsonLocation::from(&LazyLocation::new())));
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

pub struct LocationSegmentIterator<'a> {
    iter: std::vec::IntoIter<LocationSegment<'a>>,
}

impl<'a> Iterator for LocationSegmentIterator<'a> {
    type Item = LocationSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a> IntoIterator for &'a Location {
    type Item = LocationSegment<'a>;
    type IntoIter = LocationSegmentIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            iter: self
                .as_str()
                .split('/')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    p.parse::<usize>()
                        .map_or_else(|_| LocationSegment::Property(p), LocationSegment::Index)
                })
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}
