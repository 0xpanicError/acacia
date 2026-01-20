//! BTT tree output rendering

use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

use crate::tree::TreeNode;

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Render a tree to BTT format and write to file
pub fn render_tree(tree: &TreeNode, output_path: &Path) -> Result<(), OutputError> {
    let content = render_to_string(tree);

    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(output_path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Render a tree to a string in BTT format
pub fn render_to_string(tree: &TreeNode) -> String {
    let mut output = String::new();
    render_node(tree, &mut output, "", true);
    output
}

fn render_node(node: &TreeNode, output: &mut String, prefix: &str, is_last: bool) {
    match node {
        TreeNode::Root { name, children } => {
            output.push_str(name);
            output.push('\n');

            for (i, child) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                render_node(child, output, "", is_last_child);
            }
        }

        TreeNode::Branch { label, children } => {
            let connector = if is_last { "└── " } else { "├── " };
            output.push_str(prefix);
            output.push_str(connector);
            output.push_str(label);
            output.push('\n');

            let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

            for (i, child) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                render_node(child, output, &child_prefix, is_last_child);
            }
        }

        TreeNode::Leaf { label } => {
            let connector = if is_last { "└── " } else { "├── " };
            output.push_str(prefix);
            output.push_str(connector);
            output.push_str(label);
            output.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tree_rendering() {
        let tree = TreeNode::Root {
            name: "increment".to_string(),
            children: vec![
                TreeNode::Branch {
                    label: "when msg.sender is not owner".to_string(),
                    children: vec![TreeNode::Leaf {
                        label: "it should revert".to_string(),
                    }],
                },
                TreeNode::Branch {
                    label: "when msg.sender is owner".to_string(),
                    children: vec![TreeNode::Leaf {
                        label: "it should succeed".to_string(),
                    }],
                },
            ],
        };

        let output = render_to_string(&tree);
        let expected = "\
increment
├── when msg.sender is not owner
│   └── it should revert
└── when msg.sender is owner
    └── it should succeed
";
        assert_eq!(output, expected);
    }
}
