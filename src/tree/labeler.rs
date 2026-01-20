//! Condition to human-readable label conversion

use crate::analysis::{BinaryOp, ConditionContext, ConditionExpr};

/// Converts condition expressions to human-readable labels
pub struct ConditionLabeler;

impl ConditionLabeler {
    pub fn new() -> Self {
        Self
    }

    /// Generate both the "fail" and "pass" labels for a condition
    ///
    /// For `require(condition)`:
    /// - fail_label: when condition is false → reverts
    /// - pass_label: when condition is true → passes
    pub fn generate_labels(
        &self,
        condition: &ConditionExpr,
        context: ConditionContext,
        is_loop: bool,
    ) -> (String, String) {
        let prefix = match context {
            ConditionContext::Storage => "given",
            ConditionContext::External => "when",
        };

        let loop_prefix = if is_loop { "any " } else { "" };

        let (true_label, false_label) = self.expr_to_labels(condition);

        // For require(condition): if condition is false → revert
        // So fail_label is the negation (false case)
        let fail_label = format!("{} {}{}", prefix, loop_prefix, false_label);
        let pass_label = format!("{} {}{}", prefix, loop_prefix, true_label);

        (fail_label, pass_label)
    }

    /// Convert an expression to (true_case_label, false_case_label)
    fn expr_to_labels(&self, expr: &ConditionExpr) -> (String, String) {
        match expr {
            ConditionExpr::Binary { left, op, right } => {
                let (true_desc, false_desc) = match op {
                    BinaryOp::Eq => (
                        format!("{} is {}", left, self.humanize(right)),
                        format!("{} is not {}", left, self.humanize(right)),
                    ),
                    BinaryOp::NotEq => (
                        format!("{} is not {}", left, self.humanize(right)),
                        format!("{} is {}", left, self.humanize(right)),
                    ),
                    BinaryOp::Gt => (
                        format!("{} is greater than {}", left, self.humanize(right)),
                        format!("{} is at most {}", left, self.humanize(right)),
                    ),
                    BinaryOp::Gte => (
                        format!("{} is at least {}", left, self.humanize(right)),
                        format!("{} is less than {}", left, self.humanize(right)),
                    ),
                    BinaryOp::Lt => (
                        format!("{} is less than {}", left, self.humanize(right)),
                        format!("{} is at least {}", left, self.humanize(right)),
                    ),
                    BinaryOp::Lte => (
                        format!("{} is at most {}", left, self.humanize(right)),
                        format!("{} is greater than {}", left, self.humanize(right)),
                    ),
                };
                (true_desc, false_desc)
            }

            ConditionExpr::Not(inner) => {
                let (true_label, false_label) = self.expr_to_labels(inner);
                // Negation swaps the labels
                (false_label, true_label)
            }

            ConditionExpr::And(left, right) => {
                let (left_true, left_false) = self.expr_to_labels(left);
                let (right_true, right_false) = self.expr_to_labels(right);

                // a && b is true when both are true
                // a && b is false when either is false (De Morgan: !(a && b) = !a || !b)
                (
                    format!("{} and {}", left_true, right_true),
                    format!("{} or {}", left_false, right_false),
                )
            }

            ConditionExpr::Or(left, right) => {
                let (left_true, left_false) = self.expr_to_labels(left);
                let (right_true, right_false) = self.expr_to_labels(right);

                // a || b is true when either is true
                // a || b is false when both are false (De Morgan)
                (
                    format!("{} or {}", left_true, right_true),
                    format!("{} and {}", left_false, right_false),
                )
            }

            ConditionExpr::Ident(name) => {
                // Boolean identifier
                (format!("{} is true", name), format!("{} is false", name))
            }

            ConditionExpr::ExternalCall(name) => {
                (format!("{} succeeds", name), format!("{} fails", name))
            }
        }
    }

    /// Make a value more human-readable
    fn humanize(&self, value: &str) -> String {
        // Handle common cases
        match value {
            "0" => "zero".to_string(),
            "address(0)" => "zero address".to_string(),
            s if s.starts_with("Number(") => {
                // Extract number from debug format
                if let Some(num) = s.strip_prefix("Number(").and_then(|s| s.strip_suffix(")")) {
                    if num == "0" {
                        return "zero".to_string();
                    }
                    return num.to_string();
                }
                s.to_string()
            }
            _ => value.to_string(),
        }
    }
}

impl Default for ConditionLabeler {
    fn default() -> Self {
        Self::new()
    }
}
