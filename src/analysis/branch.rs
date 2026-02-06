//! Branch point data types

/// Context of a condition (determines "given" vs "when" in output)
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionContext {
    /// Storage variable - uses "given"
    Storage,
    /// External context (msg.sender, params, etc.) - uses "when"
    External,
}

/// A branch point in the control flow where a revert can occur
#[derive(Debug, Clone)]
pub struct BranchPoint {
    /// The condition expression
    pub condition: ConditionExpr,
    /// Whether this is a storage or external context condition
    pub context: ConditionContext,
    /// Whether this branch point is inside a loop (for "when any" labeling)
    pub is_loop: bool,
    /// Whether this is an external call (try/catch)
    pub is_external_call: bool,
    /// True if from if-revert pattern (TRUE causes revert), false if from require (FALSE causes revert)
    pub is_if_revert: bool,
}

/// Represents a condition expression for label generation
#[derive(Debug, Clone)]
pub enum ConditionExpr {
    /// Binary comparison: a == b, a > b, etc.
    Binary {
        left: String,
        op: BinaryOp,
        right: String,
    },
    /// Unary: !a
    Not(Box<ConditionExpr>),
    /// Logical and: a && b
    And(Box<ConditionExpr>, Box<ConditionExpr>),
    /// Logical or: a || b
    Or(Box<ConditionExpr>, Box<ConditionExpr>),
    /// Simple identifier or expression
    Ident(String),
    /// External call result
    ExternalCall(String),
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::NotEq => write!(f, "!="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Gte => write!(f, ">="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Lte => write!(f, "<="),
        }
    }
}

impl std::fmt::Display for ConditionExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConditionExpr::Binary { left, op, right } => write!(f, "{} {} {}", left, op, right),
            ConditionExpr::Not(inner) => write!(f, "!({})", inner),
            ConditionExpr::And(left, right) => write!(f, "({}) && ({})", left, right),
            ConditionExpr::Or(left, right) => write!(f, "({}) || ({})", left, right),
            ConditionExpr::Ident(s) => write!(f, "{}", s),
            ConditionExpr::ExternalCall(s) => write!(f, "{}", s),
        }
    }
}
