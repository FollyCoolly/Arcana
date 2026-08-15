use super::split_record_definition_id;
use std::collections::BTreeSet;
use std::fmt;

pub const MAX_EXPRESSION_NODES: usize = 256;
pub const MAX_EXPRESSION_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct ScoreExpression {
    root: ExpressionNode,
    record_references: BTreeSet<String>,
}

impl ScoreExpression {
    pub fn parse(source: &str) -> Result<Self, ExpressionParseError> {
        let mut parser = Parser::new(source);
        let root = parser.parse_expression(1)?;
        parser.skip_whitespace();
        if !parser.is_end() {
            return Err(parser.error("unexpected_token", "unexpected trailing input"));
        }
        if root.depth() > MAX_EXPRESSION_DEPTH {
            return Err(parser.error("expression_too_deep", "expression exceeds AST depth 32"));
        }
        Ok(Self {
            root,
            record_references: parser.record_references,
        })
    }

    pub fn record_references(&self) -> impl Iterator<Item = &str> {
        self.record_references.iter().map(String::as_str)
    }

    pub fn evaluate_raw(
        &self,
        record_value: impl Fn(&str) -> Option<f64>,
    ) -> Result<Option<f64>, ExpressionEvaluationError> {
        self.root.evaluate(&record_value)
    }

    pub fn evaluate_score(
        &self,
        record_value: impl Fn(&str) -> Option<f64>,
    ) -> Result<Option<f64>, ExpressionEvaluationError> {
        Ok(self
            .evaluate_raw(record_value)?
            .map(|value| value.clamp(0.0, 100.0)))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ExpressionNode {
    Number(f64),
    Record(String),
    Unary {
        operator: UnaryOperator,
        operand: Box<ExpressionNode>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<ExpressionNode>,
        right: Box<ExpressionNode>,
    },
    Function {
        function: Function,
        arguments: Vec<ExpressionNode>,
    },
}

impl ExpressionNode {
    fn depth(&self) -> usize {
        match self {
            Self::Number(_) | Self::Record(_) => 1,
            Self::Unary { operand, .. } => 1 + operand.depth(),
            Self::Binary { left, right, .. } => 1 + left.depth().max(right.depth()),
            Self::Function { arguments, .. } => {
                1 + arguments.iter().map(Self::depth).max().unwrap_or(0)
            }
        }
    }

    fn evaluate(
        &self,
        record_value: &impl Fn(&str) -> Option<f64>,
    ) -> Result<Option<f64>, ExpressionEvaluationError> {
        let value = match self {
            Self::Number(value) => Some(*value),
            Self::Record(id) => match record_value(id) {
                Some(value) if value.is_finite() => Some(value),
                Some(_) => return Err(ExpressionEvaluationError::NonFiniteRecord(id.clone())),
                None => None,
            },
            Self::Unary { operator, operand } => operand.evaluate(record_value)?.map(|value| {
                if *operator == UnaryOperator::Minus {
                    -value
                } else {
                    value
                }
            }),
            Self::Binary {
                operator,
                left,
                right,
            } => {
                let Some(left) = left.evaluate(record_value)? else {
                    return Ok(None);
                };
                let Some(right) = right.evaluate(record_value)? else {
                    return Ok(None);
                };
                Some(match operator {
                    BinaryOperator::Add => left + right,
                    BinaryOperator::Subtract => left - right,
                    BinaryOperator::Multiply => left * right,
                    BinaryOperator::Divide if right == 0.0 => {
                        return Err(ExpressionEvaluationError::DivisionByZero)
                    }
                    BinaryOperator::Divide => left / right,
                })
            }
            Self::Function {
                function,
                arguments,
            } => {
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let Some(value) = argument.evaluate(record_value)? else {
                        return Ok(None);
                    };
                    values.push(value);
                }
                Some(match function {
                    Function::Min => values.into_iter().fold(f64::INFINITY, f64::min),
                    Function::Max => values.into_iter().fold(f64::NEG_INFINITY, f64::max),
                    Function::Abs => values[0].abs(),
                    Function::Clamp if values[1] > values[2] => {
                        return Err(ExpressionEvaluationError::InvalidClampBounds)
                    }
                    Function::Clamp => values[0].clamp(values[1], values[2]),
                })
            }
        };

        match value {
            Some(value) if !value.is_finite() => Err(ExpressionEvaluationError::NonFiniteResult),
            value => Ok(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOperator {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Function {
    Min,
    Max,
    Abs,
    Clamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionParseError {
    pub code: &'static str,
    pub position: usize,
    pub message: String,
}

impl fmt::Display for ExpressionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at byte {}: {}",
            self.code, self.position, self.message
        )
    }
}

impl std::error::Error for ExpressionParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionEvaluationError {
    DivisionByZero,
    NonFiniteRecord(String),
    NonFiniteResult,
    InvalidClampBounds,
}

impl fmt::Display for ExpressionEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::NonFiniteRecord(id) => write!(f, "Record '{id}' is not finite"),
            Self::NonFiniteResult => write!(f, "expression produced a non-finite result"),
            Self::InvalidClampBounds => write!(f, "clamp minimum exceeds maximum"),
        }
    }
}

impl std::error::Error for ExpressionEvaluationError {}

struct Parser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    position: usize,
    node_count: usize,
    record_references: BTreeSet<String>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            position: 0,
            node_count: 0,
            record_references: BTreeSet::new(),
        }
    }

    fn parse_expression(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        self.parse_additive(depth)
    }

    fn parse_additive(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        let mut node = self.parse_multiplicative(depth)?;
        loop {
            self.skip_whitespace();
            let operator = match self.peek() {
                Some(b'+') => BinaryOperator::Add,
                Some(b'-') => BinaryOperator::Subtract,
                _ => break,
            };
            self.position += 1;
            let right = self.parse_multiplicative(depth + 1)?;
            node = self.node(
                depth,
                ExpressionNode::Binary {
                    operator,
                    left: Box::new(node),
                    right: Box::new(right),
                },
            )?;
        }
        Ok(node)
    }

    fn parse_multiplicative(
        &mut self,
        depth: usize,
    ) -> Result<ExpressionNode, ExpressionParseError> {
        let mut node = self.parse_unary(depth)?;
        loop {
            self.skip_whitespace();
            let operator = match self.peek() {
                Some(b'*') => BinaryOperator::Multiply,
                Some(b'/') => BinaryOperator::Divide,
                _ => break,
            };
            self.position += 1;
            let right = self.parse_unary(depth + 1)?;
            node = self.node(
                depth,
                ExpressionNode::Binary {
                    operator,
                    left: Box::new(node),
                    right: Box::new(right),
                },
            )?;
        }
        Ok(node)
    }

    fn parse_unary(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        self.ensure_depth(depth)?;
        self.skip_whitespace();
        let operator = match self.peek() {
            Some(b'+') => Some(UnaryOperator::Plus),
            Some(b'-') => Some(UnaryOperator::Minus),
            _ => None,
        };
        if let Some(operator) = operator {
            self.position += 1;
            let operand = self.parse_unary(depth + 1)?;
            return self.node(
                depth,
                ExpressionNode::Unary {
                    operator,
                    operand: Box::new(operand),
                },
            );
        }
        self.parse_primary(depth)
    }

    fn parse_primary(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        self.ensure_depth(depth)?;
        self.skip_whitespace();
        match self.peek() {
            Some(b'0'..=b'9') => self.parse_number(depth),
            Some(b'(') => {
                self.position += 1;
                let node = self.parse_expression(depth + 1)?;
                self.expect(b')', "expected_closing_parenthesis", "expected ')'")?;
                Ok(node)
            }
            Some(byte) if byte.is_ascii_alphabetic() || byte == b'_' => self.parse_function(depth),
            Some(_) => Err(self.error("unexpected_token", "expected a number, function, or '('")),
            None => Err(self.error("unexpected_end", "expression ended unexpectedly")),
        }
    }

    fn parse_number(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        let start = self.position;
        self.consume_digits();
        if self.peek() == Some(b'.') {
            self.position += 1;
            let fraction_start = self.position;
            self.consume_digits();
            if self.position == fraction_start {
                return Err(self.error("invalid_number", "fraction requires at least one digit"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            let exponent_start = self.position;
            self.consume_digits();
            if self.position == exponent_start {
                return Err(self.error("invalid_number", "exponent requires at least one digit"));
            }
        }
        let value = self.source[start..self.position]
            .parse::<f64>()
            .map_err(|_| self.error("invalid_number", "invalid numeric literal"))?;
        if !value.is_finite() {
            return Err(self.error("non_finite_number", "numeric literal must be finite"));
        }
        self.node(depth, ExpressionNode::Number(value))
    }

    fn parse_function(&mut self, depth: usize) -> Result<ExpressionNode, ExpressionParseError> {
        let name = self.parse_identifier();
        self.expect(
            b'(',
            "expected_function_arguments",
            "expected '(' after function name",
        )?;
        if name == "record" {
            self.skip_whitespace();
            let id = self.parse_record_id_literal()?;
            self.expect(
                b')',
                "expected_closing_parenthesis",
                "expected ')' after Record id",
            )?;
            self.record_references.insert(id.clone());
            return self.node(depth, ExpressionNode::Record(id));
        }

        let function = match name {
            "min" => Function::Min,
            "max" => Function::Max,
            "abs" => Function::Abs,
            "clamp" => Function::Clamp,
            _ => return Err(self.error("unknown_function", format!("unknown function '{name}'"))),
        };
        let mut arguments = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(b')') {
                self.position += 1;
                break;
            }
            arguments.push(self.parse_expression(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b')') => {
                    self.position += 1;
                    break;
                }
                _ => {
                    return Err(self.error(
                        "expected_argument_separator",
                        "expected ',' or ')' after function argument",
                    ))
                }
            }
        }
        let valid_arity = match function {
            Function::Min | Function::Max => arguments.len() >= 2,
            Function::Abs => arguments.len() == 1,
            Function::Clamp => arguments.len() == 3,
        };
        if !valid_arity {
            return Err(self.error(
                "invalid_function_arity",
                "function has the wrong number of arguments",
            ));
        }
        self.node(
            depth,
            ExpressionNode::Function {
                function,
                arguments,
            },
        )
    }

    fn parse_record_id_literal(&mut self) -> Result<String, ExpressionParseError> {
        if self.peek() != Some(b'\'') {
            return Err(self.error(
                "record_id_not_literal",
                "record() requires a single-quoted static id",
            ));
        }
        self.position += 1;
        let start = self.position;
        while let Some(byte) = self.peek() {
            if byte == b'\'' {
                break;
            }
            if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.'))
            {
                return Err(self.error(
                    "invalid_record_id",
                    "Record id contains an invalid character",
                ));
            }
            self.position += 1;
        }
        if self.peek() != Some(b'\'') {
            return Err(self.error("unterminated_record_id", "unterminated Record id literal"));
        }
        let id = self.source[start..self.position].to_string();
        self.position += 1;
        if split_record_definition_id(&id).is_none() {
            return Err(self.error(
                "invalid_record_id",
                "Record id must be <namespace>.<name> using lowercase snake_case",
            ));
        }
        Ok(id)
    }

    fn parse_identifier(&mut self) -> &'a str {
        let start = self.position;
        while self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.position += 1;
        }
        &self.source[start..self.position]
    }

    fn consume_digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
    }

    fn expect(
        &mut self,
        expected: u8,
        code: &'static str,
        message: &'static str,
    ) -> Result<(), ExpressionParseError> {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(code, message))
        }
    }

    fn node(
        &mut self,
        depth: usize,
        node: ExpressionNode,
    ) -> Result<ExpressionNode, ExpressionParseError> {
        self.ensure_depth(depth)?;
        self.node_count += 1;
        if self.node_count > MAX_EXPRESSION_NODES {
            return Err(self.error("expression_too_complex", "expression exceeds 256 AST nodes"));
        }
        Ok(node)
    }

    fn ensure_depth(&self, depth: usize) -> Result<(), ExpressionParseError> {
        if depth > MAX_EXPRESSION_DEPTH {
            Err(self.error("expression_too_deep", "expression exceeds AST depth 32"))
        } else {
            Ok(())
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn is_end(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn error(&self, code: &'static str, message: impl Into<String>) -> ExpressionParseError {
        ExpressionParseError {
            code,
            position: self.position,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_evaluates_documented_expression() {
        let expression = ScoreExpression::parse(
            "min(record('health.bmi') / 18.5, 1, 24.9 / record('health.bmi')) * 100",
        )
        .unwrap();
        assert_eq!(
            expression.record_references().collect::<Vec<_>>(),
            vec!["health.bmi"]
        );
        assert_eq!(
            expression
                .evaluate_score(|id| (id == "health.bmi").then_some(22.0))
                .unwrap(),
            Some(100.0)
        );
    }

    #[test]
    fn missing_record_propagates_null() {
        let expression = ScoreExpression::parse("record('health.bmi') * 2").unwrap();
        assert_eq!(expression.evaluate_score(|_| None).unwrap(), None);
    }

    #[test]
    fn rejects_dynamic_code_and_unknown_functions() {
        assert!(ScoreExpression::parse("record(variable)").is_err());
        assert!(ScoreExpression::parse("system('rm -rf')").is_err());
        assert!(ScoreExpression::parse("1 > 0").is_err());
    }

    #[test]
    fn division_by_zero_is_an_error() {
        let expression = ScoreExpression::parse("1 / record('test.zero')").unwrap();
        assert_eq!(
            expression.evaluate_score(|_| Some(0.0)),
            Err(ExpressionEvaluationError::DivisionByZero)
        );
    }
}
