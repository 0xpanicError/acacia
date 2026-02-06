use acacia::parser::SolarParser;
use acacia::tree::TreeBuilder;
use std::fs;
use std::path::PathBuf;

// Helper to set up a test file
fn setup_test_file(name: &str, content: &str) -> PathBuf {
    let mut path = PathBuf::from("testdata");
    path.push(name);
    fs::create_dir_all("testdata").unwrap();
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_library_external_calls() {
    let content = r#"
    contract TestContract {
        function testSendValue(address payable recipient, uint256 amount) public {
            Address.sendValue(recipient, amount);
        }
        
        function testFunctionCall(address target, bytes memory data) public {
            Address.functionCall(target, data);
        }
    }
    "#;

    let path = setup_test_file("LibraryCalls.sol", content);

    // Note: We need a mock setup for FoundryProject, or we can use a dummy one if parser supports it.
    // However, the parser relies on finding the file.
    // The current tests in `cli.rs` rely on `FoundryProject::discover`, which might fail in CI if not in a foundry root.
    // But unit tests usually run in the crate root.
    // Let's see if we can instantiate SolarParser without a full project or point it to current dir.

    // Acacia's SolarParser needs a FoundryProject mainly for inheritance resolution.
    // For single file tests, passing a dummy might work if we don't need inheritance.
    // But `SolarParser::new` takes `&FoundryProject`.

    // We can try to mock it or just assume we are in a valid project context or skip inheritance in this test.
    // Given the difficulty of mocking a full project structure in this environment quickly,
    // and since I cannot modify `FoundryProject` easily to be mockable without larger changes,
    // I will try to rely on the fact that `extract_branch_points_stmt` logic is unit-testable if I could access it.
    // But it's private.

    // Alternative: Create a minimal parsing test that uses `solar-parse` directly if possible, or use the existing integration flow.
    // The integration flow requires `FoundryProject`.
    // Let's create a minimal `foundry.toml` in `testdata` to make it a valid project?

    fs::write("testdata/foundry.toml", "[profile.default]").unwrap();
    let project = acacia::foundry::FoundryProject::discover().unwrap_or_else(|_| {
        acacia::foundry::FoundryProject {
            root: PathBuf::from("."),
            src_dir: PathBuf::from("src"),
            lib_dirs: vec![],
            remappings: vec![],
        }
    });

    let parser = SolarParser::new(&project);

    // Test Address.sendValue
    let ctx = parser
        .parse_function_with_inheritance(&path, "TestContract", "testSendValue")
        .unwrap();
    assert!(ctx
        .branch_points
        .iter()
        .any(|bp| bp.is_external_call && bp.condition.to_string().contains("Address.sendValue")));

    // Test Address.functionCall
    let ctx = parser
        .parse_function_with_inheritance(&path, "TestContract", "testFunctionCall")
        .unwrap();
    assert!(
        ctx.branch_points
            .iter()
            .any(|bp| bp.is_external_call
                && bp.condition.to_string().contains("Address.functionCall"))
    );
}

#[test]
fn test_safe_erc20_calls() {
    let content = r#"
    contract TestToken {
        using SafeERC20 for IERC20;
        IERC20 token;
        
        function testSafeTransfer(address to, uint256 amount) public {
            token.safeTransfer(to, amount);
        }
        
        function testTransfer(address to, uint256 amount) public {
            token.transfer(to, amount);
        }
    }
    "#;

    let path = setup_test_file("SafeERC20Calls.sol", content);

    // Mock project setup
    let project = acacia::foundry::FoundryProject {
        root: PathBuf::from("."),
        src_dir: PathBuf::from("src"),
        lib_dirs: vec![],
        remappings: vec![],
    };
    let parser = SolarParser::new(&project);

    // Test safeTransfer
    let ctx = parser
        .parse_function_with_inheritance(&path, "TestToken", "testSafeTransfer")
        .expect("Failed to parse testSafeTransfer");
    assert!(
        ctx.branch_points.iter().any(
            |bp| bp.is_external_call && bp.condition.to_string().contains("token.safeTransfer")
        ),
        "Should detect token.safeTransfer"
    );

    // Test transfer
    let ctx = parser
        .parse_function_with_inheritance(&path, "TestToken", "testTransfer")
        .expect("Failed to parse testTransfer");
    assert!(
        ctx.branch_points
            .iter()
            .any(|bp| bp.is_external_call && bp.condition.to_string().contains("token.transfer")),
        "Should detect token.transfer"
    );
}

#[test]
fn test_native_transfer() {
    let content = r#"
    contract TestNative {
        function testEthTransfer(address payable recipient, uint256 amount) public {
            recipient.transfer(amount);
        }
    }
    "#;

    let path = setup_test_file("NativeCalls.sol", content);

    // Mock project setup
    let project = acacia::foundry::FoundryProject {
        root: PathBuf::from("."),
        src_dir: PathBuf::from("src"),
        lib_dirs: vec![],
        remappings: vec![],
    };
    let parser = SolarParser::new(&project);

    let ctx = parser
        .parse_function_with_inheritance(&path, "TestNative", "testEthTransfer")
        .unwrap();
    assert!(ctx
        .branch_points
        .iter()
        .any(|bp| bp.is_external_call && bp.condition.to_string().contains("recipient.transfer")));
}
