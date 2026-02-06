//! BTT tree builder

#![allow(dead_code)]

use super::labeler::ConditionLabeler;
use crate::analysis::BranchPoint;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreeError {
    #[error("Failed to build tree: {0}")]
    BuildError(String),
}

/// A node in the BTT tree
#[derive(Debug, Clone)]
pub enum TreeNode {
    /// Root node with function name
    Root {
        name: String,
        children: Vec<TreeNode>,
    },
    /// Branch node with condition
    Branch {
        label: String,
        children: Vec<TreeNode>,
    },
    /// Leaf node with outcome
    Leaf { label: String },
}

/// Builds a BTT tree from branch points
pub struct TreeBuilder;

impl TreeBuilder {
    /// Build a tree from a function name and its branch points
    pub fn build(
        function_name: &str,
        branch_points: Vec<BranchPoint>,
    ) -> Result<TreeNode, TreeError> {
        let labeler = ConditionLabeler::new();

        // Build tree recursively from branch points
        let children = Self::build_branches(&branch_points, 0, &labeler);

        Ok(TreeNode::Root {
            name: function_name.to_string(),
            children,
        })
    }

    fn build_branches(
        branch_points: &[BranchPoint],
        index: usize,
        labeler: &ConditionLabeler,
    ) -> Vec<TreeNode> {
        if index >= branch_points.len() {
            // No more branch points - this is the success path
            return vec![TreeNode::Leaf {
                label: "it should succeed".to_string(),
            }];
        }

        let bp = &branch_points[index];

        // Generate labels for both paths
        let (fail_label, pass_label) =
            labeler.generate_labels(&bp.condition, bp.context.clone(), bp.is_loop);

        // Handle external calls specially
        if bp.is_external_call {
            let call_name = match &bp.condition {
                crate::analysis::ConditionExpr::ExternalCall(name) => name.clone(),
                _ => "external call".to_string(),
            };

            return vec![
                TreeNode::Branch {
                    label: format!("when {} fails", call_name),
                    children: vec![TreeNode::Leaf {
                        label: "it should revert".to_string(),
                    }],
                },
                TreeNode::Branch {
                    label: format!("when {} succeeds", call_name),
                    children: Self::build_branches(branch_points, index + 1, labeler),
                },
            ];
        }

        // For if-revert: condition TRUE causes revert (swap the labels)
        // For require: condition FALSE causes revert
        let (revert_label, continue_label) = if bp.is_if_revert {
            (pass_label, fail_label) // TRUE → revert, FALSE → continue
        } else {
            (fail_label, pass_label) // FALSE → revert, TRUE → continue
        };

        // Normal branch point: create two paths
        vec![
            TreeNode::Branch {
                label: revert_label,
                children: vec![TreeNode::Leaf {
                    label: "it should revert".to_string(),
                }],
            },
            TreeNode::Branch {
                label: continue_label,
                children: Self::build_branches(branch_points, index + 1, labeler),
            },
        ]
    }
}
