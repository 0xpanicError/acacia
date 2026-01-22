//! Solar AST parsing integration

#![allow(dead_code)]

use solar_parse::ast::{self, ItemKind};
use solar_parse::interface::Session;
use solar_parse::Parser;
use std::path::Path;
use thiserror::Error;

use crate::analysis::{BinaryOp, BranchPoint, ConditionContext, ConditionExpr};
use crate::foundry::FoundryProject;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Failed to parse file: {0}")]
    ParseError(String),

    #[error("Contract '{0}' not found in file")]
    ContractNotFound(String),

    #[error("Function '{0}' not found in contract '{1}'")]
    FunctionNotFound(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Context for analyzing a function - uses owned data extracted from AST
#[derive(Debug)]
pub struct FunctionContext {
    pub function_name: String,
    /// The parameter signature (e.g., "address,uint256") for distinguishing overloads
    pub signature: String,
    pub branch_points: Vec<BranchPoint>,
    pub parameters: Vec<String>,
    pub state_variables: Vec<String>,
}

/// Solar parser wrapper
pub struct SolarParser<'a> {
    project: &'a FoundryProject,
}

impl<'a> SolarParser<'a> {
    pub fn new(project: &'a FoundryProject) -> Self {
        Self { project }
    }

    /// Parse a function from a contract file and extract branch points
    pub fn parse_function(
        &self,
        file_path: &Path,
        contract_name: &str,
        function_name: &str,
    ) -> Result<FunctionContext, ParserError> {
        // Create a session for parsing
        let sess = Session::builder().with_silent_emitter(None).build();

        sess.enter(|| {
            let arena = ast::Arena::new();

            // Create parser from file
            let mut parser = Parser::from_file(&sess, &arena, file_path)
                .map_err(|e| ParserError::ParseError(format!("{:?}", e)))?;

            // Parse the file
            let source_unit = parser.parse_file().map_err(|e| {
                e.emit();
                ParserError::ParseError(file_path.display().to_string())
            })?;

            // Find the contract
            let contract = self.find_contract(&source_unit, contract_name)?;

            // Extract state variables for storage classification
            let state_vars = self.extract_state_variables(contract);

            // Find the function
            let function = self.find_function(contract, function_name)?;

            // Get function parameters
            let params = self.extract_parameters(function);

            // Extract modifier definitions for inlining
            let modifier_defs = self.extract_modifier_definitions(contract);

            // Extract branch points from modifiers and function body
            let mut branch_points = Vec::new();

            // First, extract from modifiers
            for modifier in function.header.modifiers.iter() {
                let modifier_name = modifier.name.last().as_str();
                if let Some(def) = modifier_defs.iter().find(|(name, _)| name == modifier_name) {
                    if let Some(body) = &def.1 {
                        self.extract_branch_points_from_block(
                            body,
                            &state_vars,
                            &params,
                            &mut branch_points,
                            false,
                        );
                    }
                }
            }

            // Then extract from function body
            if let Some(body) = &function.body {
                self.extract_branch_points_from_block(
                    body,
                    &state_vars,
                    &params,
                    &mut branch_points,
                    false,
                );
            }

            let signature = self.get_function_signature(function);

            Ok(FunctionContext {
                function_name: function_name.to_string(),
                signature,
                branch_points,
                parameters: params,
                state_variables: state_vars,
            })
        })
    }

    /// Parse a specific function overload by its signature (e.g., "address,uint256")
    pub fn parse_function_by_signature(
        &self,
        file_path: &Path,
        contract_name: &str,
        function_name: &str,
        signature: &str,
    ) -> Result<FunctionContext, ParserError> {
        let sess = Session::builder().with_silent_emitter(None).build();

        sess.enter(|| {
            let arena = ast::Arena::new();
            let mut parser = Parser::from_file(&sess, &arena, file_path)
                .map_err(|e| ParserError::ParseError(format!("{:?}", e)))?;

            let source_unit = parser.parse_file().map_err(|e| {
                e.emit();
                ParserError::ParseError(file_path.display().to_string())
            })?;

            let contract = self.find_contract(&source_unit, contract_name)?;
            let state_vars = self.extract_state_variables(contract);
            let function = self.find_function_by_signature(contract, function_name, signature)?;
            let params = self.extract_parameters(function);
            let modifier_defs = self.extract_modifier_definitions(contract);

            let mut branch_points = Vec::new();

            for modifier in function.header.modifiers.iter() {
                let modifier_name = modifier.name.last().as_str();
                if let Some(def) = modifier_defs.iter().find(|(name, _)| name == modifier_name) {
                    if let Some(body) = &def.1 {
                        self.extract_branch_points_from_block(
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
                self.extract_branch_points_from_block(
                    body,
                    &state_vars,
                    &params,
                    &mut branch_points,
                    false,
                );
            }

            Ok(FunctionContext {
                function_name: function_name.to_string(),
                signature: signature.to_string(),
                branch_points,
                parameters: params,
                state_variables: state_vars,
            })
        })
    }

    /// Parse all functions with the given name (returns multiple FunctionContexts for overloads)
    pub fn parse_all_functions(
        &self,
        file_path: &Path,
        contract_name: &str,
        function_name: &str,
    ) -> Result<Vec<FunctionContext>, ParserError> {
        let sess = Session::builder().with_silent_emitter(None).build();

        sess.enter(|| {
            let arena = ast::Arena::new();
            let mut parser = Parser::from_file(&sess, &arena, file_path)
                .map_err(|e| ParserError::ParseError(format!("{:?}", e)))?;

            let source_unit = parser.parse_file().map_err(|e| {
                e.emit();
                ParserError::ParseError(file_path.display().to_string())
            })?;

            let contract = self.find_contract(&source_unit, contract_name)?;
            let state_vars = self.extract_state_variables(contract);
            let modifier_defs = self.extract_modifier_definitions(contract);
            let functions = self.find_all_functions_by_name(contract, function_name);

            if functions.is_empty() {
                return Err(ParserError::FunctionNotFound(
                    function_name.to_string(),
                    contract_name.to_string(),
                ));
            }

            let mut results = Vec::new();

            for function in functions {
                let params = self.extract_parameters(function);
                let signature = self.get_function_signature(function);
                let mut branch_points = Vec::new();

                for modifier in function.header.modifiers.iter() {
                    let modifier_name = modifier.name.last().as_str();
                    if let Some(def) = modifier_defs.iter().find(|(name, _)| name == modifier_name)
                    {
                        if let Some(body) = &def.1 {
                            self.extract_branch_points_from_block(
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
                    self.extract_branch_points_from_block(
                        body,
                        &state_vars,
                        &params,
                        &mut branch_points,
                        false,
                    );
                }

                results.push(FunctionContext {
                    function_name: function_name.to_string(),
                    signature,
                    branch_points,
                    parameters: params,
                    state_variables: state_vars.clone(),
                });
            }

            Ok(results)
        })
    }

    fn find_contract<'ast>(
        &self,
        source_unit: &'ast ast::SourceUnit<'ast>,
        name: &str,
    ) -> Result<&'ast ast::ItemContract<'ast>, ParserError> {
        for item in source_unit.items.iter() {
            if let ItemKind::Contract(contract) = &item.kind {
                if contract.name.as_str() == name {
                    return Ok(contract);
                }
            }
        }
        Err(ParserError::ContractNotFound(name.to_string()))
    }

    fn find_function<'ast>(
        &self,
        contract: &'ast ast::ItemContract<'ast>,
        name: &str,
    ) -> Result<&'ast ast::ItemFunction<'ast>, ParserError> {
        for item in contract.body.iter() {
            if let ItemKind::Function(func) = &item.kind {
                if let Some(func_name) = &func.header.name {
                    if func_name.as_str() == name {
                        return Ok(func);
                    }
                }
            }
        }
        Err(ParserError::FunctionNotFound(
            name.to_string(),
            contract.name.to_string(),
        ))
    }

    /// Find all functions with the given name (for handling overloads)
    fn find_all_functions_by_name<'ast>(
        &self,
        contract: &'ast ast::ItemContract<'ast>,
        name: &str,
    ) -> Vec<&'ast ast::ItemFunction<'ast>> {
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

    /// Find a function by name and parameter signature (e.g., "address,uint256")
    fn find_function_by_signature<'ast>(
        &self,
        contract: &'ast ast::ItemContract<'ast>,
        name: &str,
        signature: &str,
    ) -> Result<&'ast ast::ItemFunction<'ast>, ParserError> {
        for item in contract.body.iter() {
            if let ItemKind::Function(func) = &item.kind {
                if let Some(func_name) = &func.header.name {
                    if func_name.as_str() == name {
                        let func_sig = self.get_function_signature(func);
                        if func_sig == signature {
                            return Ok(func);
                        }
                    }
                }
            }
        }
        Err(ParserError::FunctionNotFound(
            format!("{}({})", name, signature),
            contract.name.to_string(),
        ))
    }

    /// Extract the parameter type signature from a function (e.g., "address,uint256")
    fn get_function_signature(&self, function: &ast::ItemFunction<'_>) -> String {
        function
            .header
            .parameters
            .iter()
            .map(|p| self.type_to_string(&p.ty))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Convert a Solidity type to its string representation
    fn type_to_string(&self, ty: &ast::Type<'_>) -> String {
        use ast::TypeKind::*;
        match &ty.kind {
            Elementary(elem) => format!("{}", elem),
            Custom(path) => path.last().to_string(),
            Array(type_array) => format!("{}[]", self.type_to_string(&type_array.element)),
            Function(_) => "function".to_string(),
            Mapping(_) => "mapping".to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn extract_modifier_definitions<'ast>(
        &self,
        contract: &'ast ast::ItemContract<'ast>,
    ) -> Vec<(String, Option<&'ast ast::Block<'ast>>)> {
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

    fn extract_state_variables(&self, contract: &ast::ItemContract<'_>) -> Vec<String> {
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

    fn extract_parameters(&self, function: &ast::ItemFunction<'_>) -> Vec<String> {
        function
            .header
            .parameters
            .iter()
            .filter_map(|p| p.name.as_ref().map(|n| n.to_string()))
            .collect()
    }

    fn extract_branch_points_from_block(
        &self,
        block: &ast::Block<'_>,
        state_vars: &[String],
        params: &[String],
        branch_points: &mut Vec<BranchPoint>,
        in_loop: bool,
    ) {
        for stmt in block.stmts.iter() {
            self.extract_branch_points_from_stmt(stmt, state_vars, params, branch_points, in_loop);
        }
    }

    fn extract_branch_points_from_stmt(
        &self,
        stmt: &ast::Stmt<'_>,
        state_vars: &[String],
        params: &[String],
        branch_points: &mut Vec<BranchPoint>,
        in_loop: bool,
    ) {
        use ast::ExprKind::*;
        use ast::StmtKind::*;

        match &stmt.kind {
            // require(condition, message) or require(condition)
            Expr(expr) => {
                if let Call(callee, args) = &expr.kind {
                    if let Ident(ident) = &callee.kind {
                        let name = ident.as_str();
                        if name == "require" || name == "assert" {
                            if let Some(first_arg) = args.exprs().next() {
                                if let Some(condition) = self.expr_to_condition(first_arg) {
                                    let context =
                                        self.classify_condition(&condition, state_vars, params);
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

            // if (condition) { ... revert ... }
            If(cond, then_stmt, else_stmt) => {
                if self.stmt_contains_revert(then_stmt) {
                    if let Some(condition) = self.expr_to_condition(cond) {
                        let context = self.classify_condition(&condition, state_vars, params);
                        branch_points.push(BranchPoint {
                            condition,
                            context,
                            is_loop: in_loop,
                            is_external_call: false,
                            is_if_revert: true,
                        });
                    }
                } else {
                    self.extract_branch_points_from_stmt(
                        then_stmt,
                        state_vars,
                        params,
                        branch_points,
                        in_loop,
                    );
                }

                if let Some(else_stmt) = else_stmt {
                    self.extract_branch_points_from_stmt(
                        else_stmt,
                        state_vars,
                        params,
                        branch_points,
                        in_loop,
                    );
                }
            }

            // for loop
            For { body, .. } => {
                self.extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
            }

            // while loop
            While(_, body) => {
                self.extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
            }

            // do-while loop
            DoWhile(body, _) => {
                self.extract_branch_points_from_stmt(body, state_vars, params, branch_points, true);
            }

            // try/catch
            Try(try_stmt) => {
                let call_name = self.expr_to_string(&try_stmt.expr);
                branch_points.push(BranchPoint {
                    condition: ConditionExpr::ExternalCall(call_name),
                    context: ConditionContext::External,
                    is_loop: in_loop,
                    is_external_call: true,
                    is_if_revert: false,
                });
            }

            // Block
            Block(block) => {
                self.extract_branch_points_from_block(
                    block,
                    state_vars,
                    params,
                    branch_points,
                    in_loop,
                );
            }

            // Unchecked block
            UncheckedBlock(block) => {
                self.extract_branch_points_from_block(
                    block,
                    state_vars,
                    params,
                    branch_points,
                    in_loop,
                );
            }

            _ => {}
        }
    }

    fn stmt_contains_revert(&self, stmt: &ast::Stmt<'_>) -> bool {
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
            Block(block) => block.stmts.iter().any(|s| self.stmt_contains_revert(s)),
            _ => false,
        }
    }

    fn expr_to_condition(&self, expr: &ast::Expr<'_>) -> Option<ConditionExpr> {
        use ast::BinOpKind::*;
        use ast::ExprKind::*;

        match &expr.kind {
            Binary(left, op, right) => {
                let left_str = self.expr_to_string(left);
                let right_str = self.expr_to_string(right);

                let bin_op = match op.kind {
                    Eq => Some(BinaryOp::Eq),
                    Ne => Some(BinaryOp::NotEq),
                    Gt => Some(BinaryOp::Gt),
                    Ge => Some(BinaryOp::Gte),
                    Lt => Some(BinaryOp::Lt),
                    Le => Some(BinaryOp::Lte),
                    And => {
                        let left_cond = self.expr_to_condition(left)?;
                        let right_cond = self.expr_to_condition(right)?;
                        return Some(ConditionExpr::And(
                            Box::new(left_cond),
                            Box::new(right_cond),
                        ));
                    }
                    Or => {
                        let left_cond = self.expr_to_condition(left)?;
                        let right_cond = self.expr_to_condition(right)?;
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
                    let inner_cond = self.expr_to_condition(inner)?;
                    Some(ConditionExpr::Not(Box::new(inner_cond)))
                } else {
                    Some(ConditionExpr::Ident(self.expr_to_string(expr)))
                }
            }

            Ident(_) | Member(..) => Some(ConditionExpr::Ident(self.expr_to_string(expr))),

            _ => Some(ConditionExpr::Ident(self.expr_to_string(expr))),
        }
    }

    fn expr_to_string(&self, expr: &ast::Expr<'_>) -> String {
        use ast::ExprKind::*;

        match &expr.kind {
            Ident(ident) => ident.to_string(),
            Lit(lit, _) => format!("{:?}", lit.kind),
            Member(base, member) => {
                format!("{}.{}", self.expr_to_string(base), member.as_str())
            }
            Index(base, _kind) => {
                format!("{}[...]", self.expr_to_string(base))
            }
            Call(callee, _) => {
                format!("{}(...)", self.expr_to_string(callee))
            }
            Binary(left, op, right) => {
                format!(
                    "{} {:?} {}",
                    self.expr_to_string(left),
                    op.kind,
                    self.expr_to_string(right)
                )
            }
            _ => "expr".to_string(),
        }
    }

    fn classify_condition(
        &self,
        condition: &ConditionExpr,
        state_vars: &[String],
        params: &[String],
    ) -> ConditionContext {
        match condition {
            ConditionExpr::Binary { left, right, .. } => {
                if self.is_storage_ref(left, state_vars, params)
                    || self.is_storage_ref(right, state_vars, params)
                {
                    ConditionContext::Storage
                } else {
                    ConditionContext::External
                }
            }
            ConditionExpr::Not(inner) => self.classify_condition(inner, state_vars, params),
            ConditionExpr::And(left, right) | ConditionExpr::Or(left, right) => {
                if self.classify_condition(left, state_vars, params) == ConditionContext::Storage
                    || self.classify_condition(right, state_vars, params)
                        == ConditionContext::Storage
                {
                    ConditionContext::Storage
                } else {
                    ConditionContext::External
                }
            }
            ConditionExpr::Ident(name) => {
                if self.is_storage_ref(name, state_vars, params) {
                    ConditionContext::Storage
                } else {
                    ConditionContext::External
                }
            }
            ConditionExpr::ExternalCall(_) => ConditionContext::External,
        }
    }

    fn is_storage_ref(&self, s: &str, state_vars: &[String], params: &[String]) -> bool {
        // Check if it's a direct state variable reference
        if state_vars.iter().any(|v| {
            s == v || s.starts_with(&format!("{}.", v)) || s.starts_with(&format!("{}[", v))
        }) {
            return true;
        }

        // External context prefixes
        let external_prefixes = ["msg.", "block.", "tx."];
        if external_prefixes.iter().any(|p| s.starts_with(p)) {
            return false;
        }

        // Check if it's a parameter
        if params
            .iter()
            .any(|p| s == p || s.starts_with(&format!("{}.", p)))
        {
            return false;
        }

        false
    }

    #[allow(dead_code)]
    pub fn project(&self) -> &FoundryProject {
        self.project
    }
}
