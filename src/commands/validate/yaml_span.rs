#![allow(clippy::needless_continue)]
use std::sync::Arc;

use bon::bon;
use jsonschema::paths::LocationSegment;
use miette::SourceSpan;
use yaml_rust2::{
    Event,
    parser::{MarkedEventReceiver, Parser},
    scanner::Marker,
};

#[cfg(not(test))]
use log::{debug, trace};
#[cfg(test)]
use std::eprintln as trace;
#[cfg(test)]
use std::eprintln as debug;

use super::location::Location;

mod error;

pub use error::*;

#[derive(Debug)]
pub struct YamlSpan {
    file: Arc<String>,
    event_markers: Vec<(Event, Marker)>,
}

#[bon]
impl YamlSpan {
    #[builder]
    pub fn new(file: Arc<String>) -> Result<Self, YamlSpanError> {
        let mut ys = Self {
            file,
            event_markers: Vec::default(),
        };

        let file = ys.file.clone();
        let mut parser = Parser::new_from_str(&file);

        parser.load(&mut ys, false)?;
        Ok(ys)
    }

    pub fn get_span(&self, path: &Location) -> Result<SourceSpan, YamlSpanError> {
        debug!("Searching {path}");
        let mut event_iter = self.event_markers.iter();
        let mut path_iter = path.into_iter();

        YamlCrawler::builder()
            .events(&mut event_iter)
            .path(&mut path_iter)
            .build()
            .get_span()
    }
}

impl MarkedEventReceiver for YamlSpan {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        self.event_markers.push((ev, mark));
    }
}

struct YamlCrawler<'a, 'b, I, P>
where
    I: Iterator<Item = &'a (Event, Marker)>,
    P: Iterator<Item = LocationSegment<'b>>,
{
    events: &'a mut I,
    path: &'b mut P,
}

#[bon]
impl<'a, 'b, I, P> YamlCrawler<'a, 'b, I, P>
where
    I: Iterator<Item = &'a (Event, Marker)>,
    P: Iterator<Item = LocationSegment<'b>>,
{
    #[builder]
    pub const fn new(events: &'a mut I, path: &'b mut P) -> Self {
        Self { events, path }
    }

    pub fn get_span(&mut self) -> Result<SourceSpan, YamlSpanError> {
        let mut stream_start = false;
        let mut document_start = false;

        let Some(key) = self.path.next() else {
            let (_, marker) = self
                .events
                .find(|(e, _)| matches!(e, Event::StreamStart))
                .unwrap();
            return Ok((marker.index(), 1).into());
        };

        Ok(loop {
            let (event, _) = self.events.next().expect("Need events");
            match event {
                Event::StreamStart if !stream_start && !document_start => {
                    stream_start = true;
                }
                Event::DocumentStart if stream_start && !document_start => {
                    document_start = true;
                }
                Event::MappingStart(_, _) if stream_start && document_start => {
                    break self.key(key)?.into();
                }
                event => return Err(YamlSpanError::UnexpectedEvent(event.to_owned())),
            }
        })
    }

    fn key(&mut self, expected_key: LocationSegment<'_>) -> Result<(usize, usize), YamlSpanError> {
        trace!("Looking for location {expected_key:?}");

        loop {
            let (event, marker) = self.events.next().unwrap();
            trace!("{event:?} {marker:?}");

            match (event, expected_key) {
                (Event::Scalar(key, _, _, _), LocationSegment::Property(expected_key))
                    if key == expected_key =>
                {
                    trace!("Found matching key '{key}'");
                    break self.value();
                }
                (Event::Scalar(key, _, _, _), LocationSegment::Property(expected_key))
                    if key != expected_key =>
                {
                    trace!("Non-matching key '{key}'");
                    let (event, marker) = self.events.next().unwrap();

                    match event {
                        Event::Scalar(_, _, _, _) => continue,
                        Event::MappingStart(_, _) => self.skip_mapping(marker.index()),
                        Event::SequenceStart(_, _) => self.skip_sequence(marker.index()),
                        _ => unreachable!("{event:?}"),
                    };
                }
                (Event::Scalar(key, _, _, _), LocationSegment::Index(index)) => {
                    return Err(YamlSpanError::ExpectIndexFoundKey {
                        key: key.to_owned(),
                        index,
                    });
                }
                (Event::SequenceStart(_, _), LocationSegment::Index(index)) => {
                    break self.sequence(index, 0);
                }
                (Event::SequenceStart(_, _), _) => {
                    self.skip_sequence(marker.index());
                }
                (Event::MappingStart(_, _), _) => {
                    self.skip_mapping(marker.index());
                }
                (Event::MappingEnd, _) => {
                    return Err(YamlSpanError::EndOfMapNoKey(expected_key.to_string()));
                }
                event => unreachable!("{event:?}"),
            }
        }
    }

    fn skip_sequence(&mut self, mut last_index: usize) -> usize {
        loop {
            let (event, marker) = self.events.next().unwrap();
            trace!("SKIPPING: {event:?} {marker:?}");
            match event {
                Event::SequenceEnd => break last_index,
                Event::SequenceStart(_, _) => {
                    last_index = self.skip_sequence(last_index);
                }
                Event::MappingStart(_, _) => {
                    last_index = self.skip_mapping(last_index);
                }
                Event::Scalar(value, _, _, _) => {
                    last_index = marker.index() + value.len();
                }
                _ => (),
            }
        }
    }

    fn skip_mapping(&mut self, mut last_index: usize) -> usize {
        loop {
            let (event, marker) = self.events.next().unwrap();
            trace!("SKIPPING: {event:?} {marker:?}");
            match event {
                Event::MappingEnd => break last_index,
                Event::SequenceStart(_, _) => {
                    last_index = self.skip_sequence(last_index);
                }
                Event::MappingStart(_, _) => {
                    last_index = self.skip_mapping(last_index);
                }
                Event::Scalar(value, _, _, _) => {
                    last_index = marker.index() + value.len();
                }
                _ => continue,
            }
        }
    }

    fn sequence(
        &mut self,
        index: usize,
        curr_index: usize,
    ) -> Result<(usize, usize), YamlSpanError> {
        let (event, marker) = self.events.next().expect("Need events");
        trace!("{event:?} {marker:?}");
        trace!("index: {index}, curr_index: {curr_index}");

        Ok(match event {
            Event::SequenceEnd => return Err(YamlSpanError::EndOfSequenceNoIndex(index)),
            Event::Scalar(_, _, _, _) if index > curr_index => {
                self.sequence(index, curr_index + 1)?
            }
            Event::Scalar(value, _, _, _) if index == curr_index => (marker.index(), value.len()),
            Event::MappingStart(_, _) if index > curr_index => {
                self.skip_mapping(marker.index());
                self.sequence(index, curr_index + 1)?
            }
            Event::MappingStart(_, _) if index == curr_index => {
                trace!("Found mapping at index {index}");
                match self.path.next() {
                    None => {
                        let index = marker.index();
                        (index, self.skip_mapping(index) - index)
                    }
                    Some(key) => self.key(key)?,
                }
            }
            Event::SequenceStart(_, _) if index > curr_index => {
                self.skip_sequence(marker.index());
                self.sequence(index, curr_index + 1)?
            }
            Event::SequenceStart(_, _) if index == curr_index => {
                trace!("Found sequence at index {index}");
                match self.path.next() {
                    None => {
                        let index = marker.index();
                        (index, self.skip_sequence(index) - index)
                    }
                    Some(key) => self.key(key)?,
                }
            }
            event => unreachable!("{event:?}"),
        })
    }

    fn value(&mut self) -> Result<(usize, usize), YamlSpanError> {
        let (event, marker) = self.events.next().unwrap();
        trace!("{event:?} {marker:?}");
        let key = self.path.next();
        trace!("{key:?}");

        Ok(match (event, key) {
            (Event::Scalar(value, _, _, _), None) => (marker.index(), value.len()),
            (Event::Scalar(value, _, _, _), Some(segment)) => {
                return Err(YamlSpanError::UnexpectedScalar {
                    value: value.to_owned(),
                    segment: segment.to_string(),
                });
            }
            (Event::MappingStart(_, _), Some(LocationSegment::Property(key))) => {
                self.key(LocationSegment::Property(key))?
            }
            (Event::MappingStart(_, _), None) => {
                let index = marker.index();
                (index, self.skip_mapping(index) - index)
            }
            (Event::SequenceStart(_, _), Some(LocationSegment::Index(index))) => {
                self.sequence(index, 0)?
            }
            (Event::SequenceStart(_, _), None) => {
                let index = marker.index();
                (index, self.skip_sequence(index) - index)
            }
            event => unreachable!("{event:?}"),
        })
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use miette::{LabeledSpan, miette};
    use rstest::rstest;

    use crate::commands::validate::location::Location;

    use super::YamlSpan;

    const RECIPE: &str = include_str!("../../../test-files/recipes/recipe-pass.yml");
    const RECIPE_INVALID: &str = include_str!("../../../test-files/recipes/recipe-fail.yml");

    #[rstest]
    #[case("test: value", "", (0, 1))]
    #[case("test: value", "/test", (6, 5))]
    #[case(RECIPE, "/description", (109, 29))]
    #[case(RECIPE, "/image-version", (199, 6))]
    #[case(RECIPE, "/modules/4/source", (761, 5))]
    #[case(RECIPE, "/modules/8/from", (1040, 11))]
    #[case(RECIPE_INVALID, "/image-version", (199, 6))]
    fn test_getspan(#[case] file: &str, #[case] path: &str, #[case] expected: (usize, usize)) {
        dbg!(path, expected);
        let file = Arc::new(file.to_owned());
        let location = Location::try_from(path).unwrap();
        dbg!(&location);

        let collector = YamlSpan::builder().file(file.clone()).build().unwrap();
        let source_span = collector.get_span(&location).unwrap();
        println!(
            "{:?}",
            miette!(
                labels = [LabeledSpan::underline(source_span)],
                "Found value at {path}"
            )
            .with_source_code(file)
        );
        assert_eq!(source_span, expected.into());
    }

    #[rstest]
    #[case("test: value", "/2")]
    #[case("test: value", "/mapping")]
    #[case(RECIPE, "/test")]
    #[case(RECIPE, "/image-version/2")]
    #[case(RECIPE, "/modules/13")]
    fn test_getspan_err(#[case] file: &str, #[case] path: &str) {
        let file = Arc::new(file.to_owned());
        let location = Location::try_from(path).unwrap();
        dbg!(&location);

        let collector = YamlSpan::builder().file(file).build().unwrap();
        let source_span = collector.get_span(&location).unwrap_err();
        eprintln!("{source_span:?}");
    }
}
