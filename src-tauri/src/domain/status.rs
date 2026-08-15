use super::{
    is_snake_case_id, is_sorted_unique, split_scoped_id, ScoreExpression, Validate,
    ValidationResult, Validator,
};
use serde::{Deserialize, Serialize};

pub const STATUS_LEVEL_COUNT: usize = 5;
pub const STATUS_THRESHOLD_COUNT: usize = 4;
pub const MAX_EXPRESSION_BYTES: usize = 2048;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionDefinition {
    pub id: String,
    pub name: String,
    pub level_titles: [String; STATUS_LEVEL_COUNT],
    pub level_thresholds: [f64; STATUS_THRESHOLD_COUNT],
    pub scores: Vec<ScoreDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreDefinition {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub expression: String,
}

impl Validate for ScoreDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            is_snake_case_id(&self.id),
            "invalid_score_id",
            "id",
            "Score id must use lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        validator.require(
            self.weight.is_finite() && self.weight > 0.0,
            "invalid_score_weight",
            "weight",
            "weight must be finite and greater than zero",
        );
        validator.require_non_blank(&self.expression, "expression");
        validator.require(
            self.expression.len() <= MAX_EXPRESSION_BYTES,
            "expression_too_long",
            "expression",
            "expression exceeds 2048 bytes",
        );
        if !self.expression.trim().is_empty() && self.expression.len() <= MAX_EXPRESSION_BYTES {
            if let Err(error) = ScoreExpression::parse(&self.expression) {
                validator.error("invalid_score_expression", "expression", error.to_string());
            }
        }
        validator.finish()
    }
}

impl ScoreDefinition {
    pub fn parse_expression(&self) -> Result<ScoreExpression, super::ExpressionParseError> {
        ScoreExpression::parse(&self.expression)
    }
}

impl Validate for DimensionDefinition {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            split_scoped_id(&self.id).is_some(),
            "invalid_dimension_id",
            "id",
            "must be <pack_id>::<local_id> using lowercase snake_case",
        );
        validator.require_non_blank(&self.name, "name");
        for (index, title) in self.level_titles.iter().enumerate() {
            validator.require_non_blank(title, &format!("level_titles[{index}]"));
        }

        let thresholds_valid = self
            .level_thresholds
            .iter()
            .all(|value| value.is_finite() && *value > 0.0 && *value <= 100.0)
            && self
                .level_thresholds
                .windows(2)
                .all(|pair| pair[0] < pair[1]);
        validator.require(
            thresholds_valid,
            "invalid_level_thresholds",
            "level_thresholds",
            "thresholds must be finite and satisfy 0 < t2 < t3 < t4 < t5 <= 100",
        );
        validator.require(
            !self.scores.is_empty(),
            "empty_scores",
            "scores",
            "Dimension must contain at least one Score",
        );
        validator.require(
            is_sorted_unique(&self.scores, |score| score.id.as_str()),
            "scores_not_sorted_unique",
            "scores",
            "scores must be unique and sorted by id",
        );
        for (index, score) in self.scores.iter().enumerate() {
            validator.merge(&format!("scores[{index}]"), score.validate());
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionFile {
    pub dimensions: Vec<DimensionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusDimensionSelection {
    pub position: u8,
    pub dimension_id: String,
}

impl Validate for StatusDimensionSelection {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            self.position < 5,
            "invalid_dimension_position",
            "position",
            "position must be between 0 and 4",
        );
        validator.require(
            split_scoped_id(&self.dimension_id).is_some(),
            "invalid_dimension_id",
            "dimension_id",
            "must be a valid Pack Dimension id",
        );
        validator.finish()
    }
}

impl Validate for DimensionFile {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.require(
            !self.dimensions.is_empty(),
            "empty_file",
            "dimensions",
            "dimensions.json must be omitted instead of storing an empty array",
        );
        validator.require(
            is_sorted_unique(&self.dimensions, |dimension| dimension.id.as_str()),
            "dimensions_not_sorted_unique",
            "dimensions",
            "dimensions must be unique and sorted by id",
        );
        for (index, dimension) in self.dimensions.iter().enumerate() {
            validator.merge(&format!("dimensions[{index}]"), dimension.validate());
        }
        validator.finish()
    }
}

pub fn aggregate_dimension_score<'a>(
    scores: impl IntoIterator<Item = (&'a ScoreDefinition, Option<f64>)>,
) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut available_weight = 0.0;

    for (definition, value) in scores {
        let Some(value) = value.filter(|value| value.is_finite()) else {
            continue;
        };
        weighted_sum += value.clamp(0.0, 100.0) * definition.weight;
        available_weight += definition.weight;
    }

    (available_weight > 0.0).then(|| (weighted_sum / available_weight).clamp(0.0, 100.0))
}

pub fn dimension_level(score: Option<f64>, thresholds: &[f64; STATUS_THRESHOLD_COUNT]) -> u8 {
    let Some(score) = score.filter(|score| score.is_finite() && *score > 0.0) else {
        return 0;
    };
    1 + thresholds
        .iter()
        .take_while(|threshold| score >= **threshold)
        .count() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(id: &str, weight: f64) -> ScoreDefinition {
        ScoreDefinition {
            id: id.to_string(),
            name: id.to_string(),
            weight,
            expression: "record('test.value')".to_string(),
        }
    }

    #[test]
    fn aggregate_ignores_missing_scores_and_clamps_values() {
        let left = score("left", 1.0);
        let right = score("right", 3.0);
        assert_eq!(
            aggregate_dimension_score([(&left, Some(120.0)), (&right, None)]),
            Some(100.0)
        );
    }

    #[test]
    fn status_level_zero_is_reserved_for_missing_or_zero() {
        let thresholds = [25.0, 50.0, 75.0, 90.0];
        assert_eq!(dimension_level(None, &thresholds), 0);
        assert_eq!(dimension_level(Some(0.0), &thresholds), 0);
        assert_eq!(dimension_level(Some(0.1), &thresholds), 1);
        assert_eq!(dimension_level(Some(90.0), &thresholds), 5);
    }
}
