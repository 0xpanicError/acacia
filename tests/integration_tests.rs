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
