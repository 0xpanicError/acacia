//! Integration tests for Acacia BTT tree generation

mod common;

use common::generate_tree_for_function;

#[test]
fn test_simple_require() {
    let tree = generate_tree_for_function("SimpleRequire", "transfer");

    // Simple require(amount > 0)
    let expected = r#"transfer
├── when amount is at most zero
│   └── it should revert
└── when amount is greater than zero
    └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_with_modifier() {
    let tree = generate_tree_for_function("WithModifier", "mint");

    // "owner" is a state variable, so msg.sender == owner uses "given"
    let expected = r#"mint
├── given msg.sender is not owner
│   └── it should revert
└── given msg.sender is owner
    ├── when amount is at most zero
    │   └── it should revert
    └── when amount is greater than zero
        └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_multiple_requires() {
    let tree = generate_tree_for_function("MultipleRequires", "withdraw");

    // balances is a state variable, so uses "given"
    // address(this).balance is an expression that gets simplified
    let expected = r#"withdraw
├── when amount is at most zero
│   └── it should revert
└── when amount is greater than zero
    ├── given balances[...] is less than amount
    │   └── it should revert
    └── given balances[...] is at least amount
        ├── when expr(...).balance is less than amount
        │   └── it should revert
        └── when expr(...).balance is at least amount
            └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_if_revert() {
    let tree = generate_tree_for_function("IfRevert", "mint");

    // if (condition) revert pattern - condition being TRUE causes revert
    // So for: if (amount == 0) revert -> "when amount is zero" causes revert
    // totalSupply and maxSupply are state variables
    let expected = r#"mint
├── when amount is zero
│   └── it should revert
└── when amount is not zero
    ├── given totalSupply Add amount is greater than maxSupply
    │   └── it should revert
    └── given totalSupply Add amount is at most maxSupply
        └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_with_loop() {
    let tree = generate_tree_for_function("WithLoop", "batchTransfer");

    // Loop conditions should use "any" prefix
    // address(0) gets parsed as an expression
    let expected = r#"batchTransfer
├── when recipients.length is not amounts.length
│   └── it should revert
└── when recipients.length is amounts.length
    ├── when any recipients[...] is expr(...)
    │   └── it should revert
    └── when any recipients[...] is not expr(...)
        ├── when any amounts[...] is at most zero
        │   └── it should revert
        └── when any amounts[...] is greater than zero
            └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_storage_condition() {
    let tree = generate_tree_for_function("StorageCondition", "doSomething");

    // Storage-based condition should use "given" prefix
    let expected = r#"doSomething
├── given paused is true
│   └── it should revert
└── given paused is false
    └── it should succeed
"#;

    assert_eq!(tree, expected);
}

// ============= Function Overloading Tests =============

#[test]
fn test_overload_specific_signature_one_param() {
    use common::generate_tree_for_function_with_signature;

    // Test transfer(address) overload
    let tree =
        generate_tree_for_function_with_signature("FunctionOverloading", "transfer", "address");

    let expected = r#"transfer
├── when to is expr(...)
│   └── it should revert
└── when to is not expr(...)
    └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_overload_specific_signature_two_params() {
    use common::generate_tree_for_function_with_signature;

    // Test transfer(address, uint256) overload
    let tree = generate_tree_for_function_with_signature(
        "FunctionOverloading",
        "transfer",
        "address,uint256",
    );

    let expected = r#"transfer
├── when to is expr(...)
│   └── it should revert
└── when to is not expr(...)
    ├── when amount is at most zero
    │   └── it should revert
    └── when amount is greater than zero
        └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_overload_specific_signature_three_params() {
    use common::generate_tree_for_function_with_signature;

    // Test transfer(address, uint256, bytes) overload
    let tree = generate_tree_for_function_with_signature(
        "FunctionOverloading",
        "transfer",
        "address,uint256,bytes",
    );

    let expected = r#"transfer
├── when to is expr(...)
│   └── it should revert
└── when to is not expr(...)
    ├── when amount is at most zero
    │   └── it should revert
    └── when amount is greater than zero
        ├── when data.length is at most zero
        │   └── it should revert
        └── when data.length is greater than zero
            └── it should succeed
"#;

    assert_eq!(tree, expected);
}

#[test]
fn test_generate_all_overloads() {
    use common::generate_trees_for_all_overloads;

    // Generate trees for all transfer overloads
    let trees = generate_trees_for_all_overloads("FunctionOverloading", "transfer");

    // Should have 3 overloads
    assert_eq!(trees.len(), 3);

    // Verify signatures are present
    let signatures: Vec<&str> = trees.iter().map(|(sig, _)| sig.as_str()).collect();
    assert!(signatures.contains(&"address"));
    assert!(signatures.contains(&"address,uint256"));
    assert!(signatures.contains(&"address,uint256,bytes"));

    // Each tree should have the function name as root
    for (_, tree) in &trees {
        assert!(tree.starts_with("transfer\n"));
    }
}

// ============= Contract-Wide Generation Tests =============

#[test]
fn test_generate_all_public_functions() {
    use common::generate_trees_for_contract;

    // Generate trees for all public/external functions in AllFunctions contract
    let trees = generate_trees_for_contract("AllFunctions");

    // Should have 3 public/external functions: externalFunc, publicFunc, pausableFunc
    // Should NOT include internalFunc or privateFunc
    assert_eq!(trees.len(), 3);

    // Verify function names are present
    let func_names: Vec<&str> = trees.iter().map(|(name, _)| name.as_str()).collect();
    assert!(func_names.contains(&"externalFunc"));
    assert!(func_names.contains(&"publicFunc"));
    assert!(func_names.contains(&"pausableFunc"));

    // Should NOT include internal/private functions
    assert!(!func_names.contains(&"internalFunc"));
    assert!(!func_names.contains(&"privateFunc"));
}
