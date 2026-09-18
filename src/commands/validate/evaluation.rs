use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Not,
};

use jsonschema::{ValidationError, error::ValidationErrorKind, paths::Location};
use serde::{Deserialize, Deserializer};

#[cfg(not(test))]
use log::debug;

#[cfg(test)]
use std::eprintln as debug;

#[derive(Debug, Clone, Deserialize)]
pub struct EvaluationList {
    pub details: Vec<EvaluationDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationDetail {
    pub valid: bool,

    #[serde(deserialize_with = "location")]
    pub instance_location: Location,

    #[serde(deserialize_with = "location")]
    pub schema_location: Location,

    #[serde(deserialize_with = "location")]
    pub evaluation_path: Location,
}

fn location<'de, D>(des: D) -> Result<Location, D::Error>
where
    D: Deserializer<'de>,
{
    let val = String::deserialize(des)?;
    let mut split = val.split('/');
    split.next();
    Ok(split.fold(Location::new(), |loc, seg| loc.join(seg)))
}

impl EvaluationList {
    // pub fn filter_errors<'slice, 'error>(
    //     &self,
    //     errs: &'slice [&'slice ValidationError<'error>],
    // ) -> Vec<&'slice ValidationError<'error>> {
    //     errs.iter().flat_map(|err| self.all_errs(err)).collect()
    // }

    pub fn all_errs<'slice, 'error>(
        &self,
        err: &'slice ValidationError<'error>,
    ) -> Vec<&'slice ValidationError<'error>> {
        // debug!("Checking instance path: {}", err.instance_path());
        match err.kind() {
            ValidationErrorKind::OneOfNotValid { context }
            | ValidationErrorKind::AnyOf { context } => {
                // debug!("Context size: {}", context.len());
                let indexes = self.find_closest_indexes(context, err);
                // debug!("Closest indexes: {indexes:?}");
                indexes
                    .into_iter()
                    .flat_map(|index| {
                        // debug!("Checking index {index}");
                        let errs = &context[index];
                        // debug!("Error count: {}", errs.len());

                        errs.iter().flat_map(|err| self.all_errs(err))
                    })
                    .collect()
            }
            _ => vec![err],
        }
    }

    // fn flatten<'slice, 'error>(
    //     &self,
    //     errs: &'slice [ValidationError<'error>],
    //     out: &'slice mut Vec<&'slice ValidationError<'error>>,
    // ) {
    //     for err in errs {}
    // }

    // fn full_len(&self, err: &ValidationError<'_>) -> usize {
    //     match err.kind() {
    //         ValidationErrorKind::AnyOf { context }
    //         | ValidationErrorKind::OneOfNotValid { context } => context
    //             [self.find_closest_index(context)]
    //         .iter()
    //         .map(|err| self.full_len(err))
    //         .sum(),
    //         _ => 1,
    //     }
    // }

    fn find_closest_indexes(
        &self,
        context: &Vec<Vec<ValidationError<'_>>>,
        parent: &ValidationError,
    ) -> Vec<usize> {
        // We must only enter this function if
        assert!(
            matches!(
                parent.kind(),
                ValidationErrorKind::AnyOf { .. } | ValidationErrorKind::OneOfNotValid { .. }
            ),
            "Must be evaluating anyOf or oneOf"
        );

        let scores = context
            .iter()
            .enumerate()
            // Get the alternate const score and error count
            .map(|(index, errs)| {
                let eval_path = &parent.evaluation_path().join(index);
                let details = self.matching_details(eval_path, parent.instance_path());
                dbg!(&details);
                let total = details.len();
                let fail_count = details.iter().filter(|detail| !detail.valid).count();
                debug!("Instance path: {}", parent.instance_path());
                debug!("Eval path: {eval_path}");
                debug!("Total detail count: {total}");
                debug!("Total detail fail count: {fail_count}");
                debug!("Total error count: {}", errs.len());

                let score = self.error_scoring(parent, errs);

                (index, score)
            })
            // Push all indexes with the same score together
            .fold(
                BTreeMap::<usize, Vec<usize>>::new(),
                |mut acc, (index, score)| {
                    acc.entry(score)
                        .and_modify(|stack| {
                            stack.push(index);
                        })
                        .or_insert_with(|| vec![index]);
                    acc
                },
            );
        debug!("Scores for {}", parent.instance_path());
        debug!("{scores:#?}");

        scores
            .into_iter()
            // Find the indexes with the lowest scores
            .min_by(|(score1, _), (score2, _)| score1.cmp(score2))
            .map(|set| {
                debug!(
                    "Found minimum for {} at {}",
                    parent.evaluation_path(),
                    parent.instance_path()
                );
                debug!("{set:#?}");
                set.1
            })
            .inspect(|indexes| {
                for index in indexes {
                    assert!(
                        *index < context.len(),
                        "Returned index must always be in the bounds of the context"
                    );
                }
            })
            .unwrap_or_default()
    }

    fn error_scoring(&self, parent: &ValidationError<'_>, errs: &[ValidationError<'_>]) -> usize {
        assert!(
            matches!(
                parent.kind(),
                ValidationErrorKind::AnyOf { .. } | ValidationErrorKind::OneOfNotValid { .. }
            ),
            "Must be evaluating anyOf or oneOf"
        );
        let mut score = 0;

        for err in errs {
            // debug!("Checking for alternative constant count for {err:#?}");
            // debug!("Instance path: {}", err.instance_path());
            // debug!("Evaluation path: {}", err.evaluation_path());

            match err.kind() {
                ValidationErrorKind::Constant { .. } => {
                    let details = self
                        .matching_details(parent.evaluation_path(), err.instance_path())
                        .into_iter()
                        .filter(|detail| {
                            detail.evaluation_path.as_str().ends_with("/const") && detail.valid
                        })
                        .collect::<Vec<_>>();
                    // debug!("Matching details:\n{details:#?}");
                    if details.is_empty().not() {
                        score += details.len();
                    }
                }
                ValidationErrorKind::AnyOf { context }
                | ValidationErrorKind::OneOfNotValid { context } => {
                    score += self
                        .find_closest_indexes(context, err)
                        .into_iter()
                        .map(|index| self.error_scoring(err, &context[index]))
                        .sum::<usize>();
                }
                _ => {
                    // score += 1;
                    // debug!("Isn't a kind to check against");
                }
            }
        }
        score
    }

    fn matching_details(
        &self,
        evaluation_path: &Location,
        instance_path: &Location,
    ) -> Vec<&EvaluationDetail> {
        self.details
            .iter()
            .filter(|detail| {
                detail
                    .evaluation_path
                    .as_str()
                    .starts_with(evaluation_path.as_str())
                    && detail
                        .instance_location
                        .as_str()
                        .starts_with(instance_path.as_str())
            })
            .collect()
    }

    // fn is_full_failure(&self, err: &ValidationError) -> bool {
    //     self.details
    //         .iter()
    //         .filter(|detail| &detail.instance_location == err.instance_path())
    //         .filter(|detail| {
    //             &detail.schema_location == err.schema_path()
    //                 && matches!(
    //                     err.kind(),
    //                     jsonschema::error::ValidationErrorKind::AnyOf { .. }
    //                         | jsonschema::error::ValidationErrorKind::OneOfNotValid { .. }
    //                 )
    //         })
    //         .all(|detail| !detail.valid)
    // }
}
