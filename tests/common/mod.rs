//! Common test utilities

use std::path::PathBuf;

// Re-implement the core logic for testing
use solar_parse::ast::{self, ItemKind};
use solar_parse::interface::Session;
use solar_parse::Parser;

/// Get the path to the testdata directory
pub fn testdata_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// Generate a BTT tree for a function in a test contract
pub fn generate_tree_for_function(contract_name: &str, function_name: &str) -> String {
    let file_path = testdata_dir().join(format!("{}.sol", contract_name));

    let sess = Session::builder().with_silent_emitter(None).build();

    sess.enter(|| {
        let arena = ast::Arena::new();
        let mut parser =
            Parser::from_file(&sess, &arena, &file_path).expect("Failed to create parser");

        let source_unit = parser
            .parse_file()
            .map_err(|e| {
                e.emit();
            })
            .expect("Failed to parse file");

        // Find contract and function
        let contract = find_contract(&source_unit, contract_name).expect("Contract not found");

        let function = find_function(contract, function_name).expect("Function not found");

        // Extract state variables
        let state_vars = extract_state_variables(contract);

        // Extract parameters
        let params = extract_parameters(function);

        // Extract modifier definitions
        let modifier_defs = extract_modifier_definitions(contract);

        // Extract branch points
        let mut branch_points = Vec::new();

        // From modifiers
        for modifier in function.header.modifiers.iter() {
            let modifier_name = modifier.name.last().as_str();
            if let Some((_, body)) = modifier_defs.iter().find(|(name, _)| name == modifier_name) {
                if let Some(body) = body {
                    extract_branch_points_from_block(
                        body,
                        &state_vars,
                        &params,
                        &mut branch_points,
                        false,
                    );
                }
            }
        }

        // From function body
        if let Some(body) = &function.body {
            extract_branch_points_from_block(body, &state_vars, &params, &mut branch_points, false);
        }

        // Build tree
        let tree = build_tree(function_name, branch_points);

        // Render to string
        render_tree(&tree)
    })
}

/// Generate a BTT tree for a specific function overload by signature
pub fn generate_tree_for_function_with_signature(
    contract_name: &str,
    function_name: &str,
    signature: &str,
) -> String {
    let file_path = testdata_dir().join(format!("{}.sol", contract_name));

    let sess = Session::builder().with_silent_emitter(None).build();

    sess.enter(|| {
        let arena = ast::Arena::new();
        let mut parser =
            Parser::from_file(&sess, &arena, &file_path).expect("Failed to create parser");

        let source_unit = parser
            .parse_file()
            .map_err(|e| {
                e.emit();
            })
            .expect("Failed to parse file");

        let contract = find_contract(&source_unit, contract_name).expect("Contract not found");
        let state_vars = extract_state_variables(contract);
        let modifier_defs = extract_modifier_definitions(contract);

        // Find function by signature
        let function = find_function_by_signature(contract, function_name, signature)
            .expect("Function with signature not found");

        let params = extract_parameters(function);
        let mut branch_points = Vec::new();

        for modifier in function.header.modifiers.iter() {
            let modifier_name = modifier.name.last().as_str();
            if let Some((_, body)) = modifier_defs.iter().find(|(name, _)| name == modifier_name) {
                if let Some(body) = body {
                    extract_branch_points_from_block(
                        body,
                        &state_vars,
                        &params,
                        &mut branch_points,
                        false,
                    );
                }
            }
        }

        if let Some(body) = &function.body {
            extract_branch_points_from_block(body, &state_vars, &params, &mut branch_points, false);
        }

        let tree = build_tree(function_name, branch_points);
        render_tree(&tree)
    })
}

/// Generate BTT trees for all overloads of a function, returning (signature, tree) pairs
pub fn generate_trees_for_all_overloads(
    contract_name: &str,
    function_name: &str,
) -> Vec<(String, String)> {
    let file_path = testdata_dir().join(format!("{}.sol", contract_name));

    let sess = Session::builder().with_silent_emitter(None).build();

    sess.enter(|| {
        let arena = ast::Arena::new();
        let mut parser =
            Parser::from_file(&sess, &arena, &file_path).expect("Failed to create parser");

        let source_unit = parser
            .parse_file()
            .map_err(|e| {
                e.emit();
            })
            .expect("Failed to parse file");

        let contract = find_contract(&source_unit, contract_name).expect("Contract not found");
        let state_vars = extract_state_variables(contract);
        let modifier_defs = extract_modifier_definitions(contract);
        let functions = find_all_functions_by_name(contract, function_name);

        let mut results = Vec::new();

        for function in functions {
            let params = extract_parameters(function);
            let signature = get_function_signature(function);
            let mut branch_points = Vec::new();

            for modifier in function.header.modifiers.iter() {
                let modifier_name = modifier.name.last().as_str();
                if let Some((_, body)) =
                    modifier_defs.iter().find(|(name, _)| name == modifier_name)
                {
                    if let Some(body) = body {
                        extract_branch_points_from_block(
                            body,
                            &state_vars,
                            &params,
                            &mut branch_points,
                            false,
                        );
                    }
                }
            }

            if let Some(body) = &function.body {
                extract_branch_points_from_block(
                    body,
                    &state_vars,
                    &params,
                    &mut branch_points,
                    false,
                );
            }

            let tree = build_tree(function_name, branch_points);
            results.push((signature, render_tree(&tree)));
        }

        results
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionContext {
    Storage,
    External,
}

#[derive(Debug, Clone)]
pub struct BranchPoint {
    pub condition: ConditionExpr,
    pub context: ConditionContext,
    pub is_loop: bool,
    pub is_external_call: bool,
    /// True if this came from an if-revert pattern (condition TRUE causes revert)
    /// False if from require/assert (condition FALSE causes revert)
    pub is_if_revert: bool,
}

#[derive(Debug, Clone)]
pub enum ConditionExpr {
    Binary {
        left: String,
        op: BinaryOp,
        right: String,
    },
    Not(Box<ConditionExpr>),
    And(Box<ConditionExpr>, Box<ConditionExpr>),
    Or(Box<ConditionExpr>, Box<ConditionExpr>),
    Ident(String),
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

fn find_contract<'a>(
    source_unit: &'a ast::SourceUnit<'a>,
    name: &str,
) -> Option<&'a ast::ItemContract<'a>> {
    for item in source_unit.items.iter() {
        if let ItemKind::Contract(contract) = &item.kind {
            if contract.name.as_str() == name {
                return Some(contract);
            }
        }
    }
    None
}

fn find_function<'a>(
    contract: &'a ast::ItemContract<'a>,
    name: &str,
) -> Option<&'a ast::ItemFunction<'a>> {
    for item in contract.body.iter() {
        if let ItemKind::Function(func) = &item.kind {
            if let Some(func_name) = &func.header.name {
                if func_name.as_str() == name {
                    return Some(func);
                }
            }
        }
    }
    None
}

fn find_all_functions_by_name<'a>(
    contract: &'a ast::ItemContract<'a>,
    name: &str,
) -> Vec<&'a ast::ItemFunction<'a>> {
    let mut functions = Vec::new();
    for item in contract.body.iter() {
        if let ItemKind::Function(func) = &item.kind {
            if let Some(func_name) = &func.header.name {
                if func_name.as_str() == name {
                    functions.push(func);
                }
            }
        }
    }
    functions
}

fn find_function_by_signature<'a>(
    contract: &'a ast::ItemContract<'a>,
    name: &str,
    signature: &str,
) -> Option<&'a ast::ItemFunction<'a>> {
    for item in contract.body.iter() {
        if let ItemKind::Function(func) = &item.kind {
            if let Some(func_name) = &func.header.name {
                if func_name.as_str() == name {
                    let func_sig = get_function_signature(func);
                    if func_sig == signature {
                        return Some(func);
                    }
                }
            }
        }
    }
    None
}

fn get_function_signature(function: &ast::ItemFunction<'_>) -> String {
    function
        .header
        .parameters
        .iter()
        .map(|p| type_to_string(&p.ty))
        .collect::<Vec<_>>()
        .join(",")
}

fn type_to_string(ty: &ast::Type<'_>) -> String {
    use ast::TypeKind::*;
    match &ty.kind {
        Elementary(elem) => format!("{}", elem),
        Custom(path) => path.last().to_string(),
        Array(type_array) => format!("{}[]", type_to_string(&type_array.element)),
        Function(_) => "function".to_string(),
        Mapping(_) => "mapping".to_string(),
        _ => "unknown".to_string(),
    }
}

fn extract_state_variables(contract: &ast::ItemContract<'_>) -> Vec<String> {
    let mut vars = Vec::new();
    for item in contract.body.iter() {
        if let ItemKind::Variable(var) = &item.kind {
            if let Some(name) = &var.name {
                vars.push(name.to_string());
            }
        }
    }
    vars
}

fn extract_parameters(function: &ast::ItemFunction<'_>) -> Vec<String> {
    function
        .header
        .parameters
        .iter()
        .filter_map(|p| p.name.as_ref().map(|n| n.to_string()))
        .collect()
}

fn extract_modifier_definitions<'a>(
    contract: &'a ast::ItemContract<'a>,
) -> Vec<(String, Option<&'a ast::Block<'a>>)> {
    let mut modifiers = Vec::new();
    for item in contract.body.iter() {
        if let ItemKind::Function(func) = &item.kind {
            if func.kind == ast::FunctionKind::Modifier {
                if let Some(name) = &func.header.name {
                    modifiers.push((name.to_string(), func.body.as_ref()));
                }
            }
        }
    }
    modifiers
}

fn extract_branch_points_from_block(
    block: &ast::Block<'_>,
    state_vars: &[String],
    params: &[String],
    branch_points: &mut Vec<BranchPoint>,
    in_loop: bool,
) {
    for stmt in block.stmts.iter() {
        extract_branch_points_from_stmt(stmt, state_vars, params, branch_points, in_loop);
    }
}

fn extract_branch_points_from_stmt(
    stmt: &ast::Stmt<'_>,
    state_vars: &[String],
    params: &[String],
    branch_points: &mut Vec<BranchPoint>,
    in_loop: bool,
) {
    use ast::ExprKind::*;
    use ast::StmtKind::*;

    match &stmt.kind {
        Expr(expr) => {
            if let Call(callee, args) = &expr.kind {
                if let Ident(ident) = &callee.kind {
                    let name = ident.as_str();
                    if name == "require" || name == "assert" {
                        if let Some(first_arg) = args.exprs().next() {
                            if let Some(condition) = expr_to_condition(first_arg) {
                                let context = classify_condition(&condition, state_vars, params);
                                branch_points.push(BranchPoint {
                                    condition,
                                    context,
                                    is_loop: in_loop,
                                    is_external_call: false,
                                    is_if_revert: false,
                                });
                            }
                        }
                    }
                }
            }
        }
        If(cond, then_stmt, else_stmt) => {
            if stmt_contains_revert(then_stmt) {
                if let Some(condition) = expr_to_condition(cond) {
                    let context = classify_condition(&condition, state_vars, params);
                    branch_points.push(BranchPoint {
                        condition,
                        context,
                        is_loop: in_loop,
                        is_external_call: false,
                        is_if_revert: true, // if-revert: TRUE causes revert
                    });
                }
            } else {
                extract_branch_points_from_stmt(
                    then_stmt,
                    state_vars,
                    params,
                    branch_points,
                    in_loop,
                );
            }
            if let Some(else_stmt) = else_stmt {
                extract_branch_points_from_stmt(
                    else_stmt,
                    state_vars,
                    params,
                    branch_points,
                    in_loop,
                );
            }
        }
        For { body, .. } => {
            extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
        }
        While(_, body) => {
            extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
        }
        DoWhile(body, _) => {
            extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
        }
        Block(block) => {
            extract_branch_points_from_block(block, state_vars, params, branch_points, in_loop);
        }
        UncheckedBlock(block) => {
            extract_branch_points_from_block(block, state_vars, params, branch_points, in_loop);
        }
        _ => {}
    }
}

fn stmt_contains_revert(stmt: &ast::Stmt<'_>) -> bool {
    use ast::ExprKind::*;
    use ast::StmtKind::*;

    match &stmt.kind {
        Revert(..) => true,
        Expr(expr) => {
            if let Call(callee, _) = &expr.kind {
                if let Ident(ident) = &callee.kind {
                    return ident.as_str() == "revert";
                }
            }
            false
        }
        Block(block) => block.stmts.iter().any(|s| stmt_contains_revert(s)),
        _ => false,
    }
}

fn expr_to_condition(expr: &ast::Expr<'_>) -> Option<ConditionExpr> {
    use ast::BinOpKind::*;
    use ast::ExprKind::*;

    match &expr.kind {
        Binary(left, op, right) => {
            let left_str = expr_to_string(left);
            let right_str = expr_to_string(right);

            let bin_op = match op.kind {
                Eq => Some(BinaryOp::Eq),
                Ne => Some(BinaryOp::NotEq),
                Gt => Some(BinaryOp::Gt),
                Ge => Some(BinaryOp::Gte),
                Lt => Some(BinaryOp::Lt),
                Le => Some(BinaryOp::Lte),
                And => {
                    let left_cond = expr_to_condition(left)?;
                    let right_cond = expr_to_condition(right)?;
                    return Some(ConditionExpr::And(
                        Box::new(left_cond),
                        Box::new(right_cond),
                    ));
                }
                Or => {
                    let left_cond = expr_to_condition(left)?;
                    let right_cond = expr_to_condition(right)?;
                    return Some(ConditionExpr::Or(Box::new(left_cond), Box::new(right_cond)));
                }
                _ => None,
            };

            bin_op.map(|op| ConditionExpr::Binary {
                left: left_str,
                op,
                right: right_str,
            })
        }
        Unary(op, inner) => {
            if op.kind == ast::UnOpKind::Not {
                let inner_cond = expr_to_condition(inner)?;
                Some(ConditionExpr::Not(Box::new(inner_cond)))
            } else {
                Some(ConditionExpr::Ident(expr_to_string(expr)))
            }
        }
        Ident(_) | Member(..) => Some(ConditionExpr::Ident(expr_to_string(expr))),
        _ => Some(ConditionExpr::Ident(expr_to_string(expr))),
    }
}

fn expr_to_string(expr: &ast::Expr<'_>) -> String {
    use ast::ExprKind::*;

    match &expr.kind {
        Ident(ident) => ident.to_string(),
        Lit(lit, _) => format!("{:?}", lit.kind),
        Member(base, member) => format!("{}.{}", expr_to_string(base), member.as_str()),
        Index(base, _) => format!("{}[...]", expr_to_string(base)),
        Call(callee, _) => format!("{}(...)", expr_to_string(callee)),
        Binary(left, op, right) => format!(
            "{} {:?} {}",
            expr_to_string(left),
            op.kind,
            expr_to_string(right)
        ),
        _ => "expr".to_string(),
    }
}

fn classify_condition(
    condition: &ConditionExpr,
    state_vars: &[String],
    params: &[String],
) -> ConditionContext {
    match condition {
        ConditionExpr::Binary { left, right, .. } => {
            if is_storage_ref(left, state_vars, params) || is_storage_ref(right, state_vars, params)
            {
                ConditionContext::Storage
            } else {
                ConditionContext::External
            }
        }
        ConditionExpr::Not(inner) => classify_condition(inner, state_vars, params),
        ConditionExpr::And(left, right) | ConditionExpr::Or(left, right) => {
            if classify_condition(left, state_vars, params) == ConditionContext::Storage
                || classify_condition(right, state_vars, params) == ConditionContext::Storage
            {
                ConditionContext::Storage
            } else {
                ConditionContext::External
            }
        }
        ConditionExpr::Ident(name) => {
            if is_storage_ref(name, state_vars, params) {
                ConditionContext::Storage
            } else {
                ConditionContext::External
            }
        }
        ConditionExpr::ExternalCall(_) => ConditionContext::External,
    }
}

fn is_storage_ref(s: &str, state_vars: &[String], params: &[String]) -> bool {
    if state_vars
        .iter()
        .any(|v| s == v || s.starts_with(&format!("{}.", v)) || s.starts_with(&format!("{}[", v)))
    {
        return true;
    }
    let external_prefixes = ["msg.", "block.", "tx."];
    if external_prefixes.iter().any(|p| s.starts_with(p)) {
        return false;
    }
    if params
        .iter()
        .any(|p| s == p || s.starts_with(&format!("{}.", p)))
    {
        return false;
    }
    false
}

// Tree building
#[derive(Debug, Clone)]
pub enum TreeNode {
    Root {
        name: String,
        children: Vec<TreeNode>,
    },
    Branch {
        label: String,
        children: Vec<TreeNode>,
    },
    Leaf {
        label: String,
    },
}

fn build_tree(function_name: &str, branch_points: Vec<BranchPoint>) -> TreeNode {
    let children = build_branches(&branch_points, 0);
    TreeNode::Root {
        name: function_name.to_string(),
        children,
    }
}

fn build_branches(branch_points: &[BranchPoint], index: usize) -> Vec<TreeNode> {
    if index >= branch_points.len() {
        return vec![TreeNode::Leaf {
            label: "it should succeed".to_string(),
        }];
    }

    let bp = &branch_points[index];
    let (fail_label, pass_label) = generate_labels(&bp.condition, bp.context.clone(), bp.is_loop);

    if bp.is_external_call {
        let call_name = match &bp.condition {
            ConditionExpr::ExternalCall(name) => name.clone(),
            _ => "external call".to_string(),
        };
        return vec![
            TreeNode::Branch {
                label: format!("given {} fails", call_name),
                children: vec![TreeNode::Leaf {
                    label: "it should revert".to_string(),
                }],
            },
            TreeNode::Branch {
                label: format!("given {} succeeds", call_name),
                children: build_branches(branch_points, index + 1),
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

    vec![
        TreeNode::Branch {
            label: revert_label,
            children: vec![TreeNode::Leaf {
                label: "it should revert".to_string(),
            }],
        },
        TreeNode::Branch {
            label: continue_label,
            children: build_branches(branch_points, index + 1),
        },
    ]
}

fn generate_labels(
    condition: &ConditionExpr,
    context: ConditionContext,
    is_loop: bool,
) -> (String, String) {
    let prefix = match context {
        ConditionContext::Storage => "given",
        ConditionContext::External => "when",
    };
    let loop_prefix = if is_loop { "any " } else { "" };
    let (true_label, false_label) = expr_to_labels(condition);
    (
        format!("{} {}{}", prefix, loop_prefix, false_label),
        format!("{} {}{}", prefix, loop_prefix, true_label),
    )
}

fn expr_to_labels(expr: &ConditionExpr) -> (String, String) {
    match expr {
        ConditionExpr::Binary { left, op, right } => {
            let right_humanized = humanize(right);
            match op {
                BinaryOp::Eq => (
                    format!("{} is {}", left, right_humanized),
                    format!("{} is not {}", left, right_humanized),
                ),
                BinaryOp::NotEq => (
                    format!("{} is not {}", left, right_humanized),
                    format!("{} is {}", left, right_humanized),
                ),
                BinaryOp::Gt => (
                    format!("{} is greater than {}", left, right_humanized),
                    format!("{} is at most {}", left, right_humanized),
                ),
                BinaryOp::Gte => (
                    format!("{} is at least {}", left, right_humanized),
                    format!("{} is less than {}", left, right_humanized),
                ),
                BinaryOp::Lt => (
                    format!("{} is less than {}", left, right_humanized),
                    format!("{} is at least {}", left, right_humanized),
                ),
                BinaryOp::Lte => (
                    format!("{} is at most {}", left, right_humanized),
                    format!("{} is greater than {}", left, right_humanized),
                ),
            }
        }
        ConditionExpr::Not(inner) => {
            let (true_label, false_label) = expr_to_labels(inner);
            (false_label, true_label)
        }
        ConditionExpr::And(left, right) => {
            let (left_true, left_false) = expr_to_labels(left);
            let (right_true, right_false) = expr_to_labels(right);
            (
                format!("{} and {}", left_true, right_true),
                format!("{} or {}", left_false, right_false),
            )
        }
        ConditionExpr::Or(left, right) => {
            let (left_true, left_false) = expr_to_labels(left);
            let (right_true, right_false) = expr_to_labels(right);
            (
                format!("{} or {}", left_true, right_true),
                format!("{} and {}", left_false, right_false),
            )
        }
        ConditionExpr::Ident(name) => (format!("{} is true", name), format!("{} is false", name)),
        ConditionExpr::ExternalCall(name) => {
            (format!("{} succeeds", name), format!("{} fails", name))
        }
    }
}

fn humanize(value: &str) -> String {
    match value {
        "0" => "zero".to_string(),
        "address(0)" => "zero address".to_string(),
        s if s.starts_with("Number(") => {
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

fn render_tree(tree: &TreeNode) -> String {
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
                render_node(child, output, "", i == children.len() - 1);
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
                render_node(child, output, &child_prefix, i == children.len() - 1);
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
