# 🌳 Acacia

**BTT Test Tree Generator for Solidity Smart Contracts**

Acacia analyzes Solidity functions and generates [Branching Tree Technique (BTT)](https://twitter.com/PaulRBerg/status/1682346315806539776) style test trees, helping you write comprehensive unit tests.

## Installation

### From crates.io (Recommended)

```bash
cargo install acacia
```

### From Source

```bash
git clone https://github.com/your-org/acacia.git
cd acacia
cargo install --path .
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/your-org/acacia/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `acacia-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `acacia-x86_64-apple-darwin.tar.gz` |
| Linux (x64) | `acacia-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x64) | `acacia-x86_64-pc-windows-msvc.zip` |

```bash
# Example: macOS Apple Silicon
curl -L https://github.com/your-org/acacia/releases/latest/download/acacia-aarch64-apple-darwin.tar.gz | tar xz
sudo mv acacia /usr/local/bin/
```

## Quick Start

```bash
# Navigate to your Foundry project
cd my-foundry-project

# Generate a test tree
acacia generate MyContract::myFunction
```

This creates `test/trees/MyContract.myFunction.tree`:

```
myFunction
├── when caller is not owner
│   └── it should revert
└── when caller is owner
    ├── when amount is zero
    │   └── it should revert
    └── when amount is greater than zero
        └── it should succeed
```

## Features

| Feature | Description |
|---------|-------------|
| **Modifier Inlining** | Traces through modifiers to include all conditions |
| **Pattern Detection** | Handles `require`, `assert`, and `if-revert` patterns |
| **Loop Awareness** | Uses "any" prefix for conditions inside loops |
| **Smart Labeling** | "given" for storage conditions, "when" for external context |

## BTT Format

Acacia follows Paul R. Berg's BTT specification:

- **"given"** - Conditions based on contract storage state
- **"when"** - Conditions based on external context (msg.sender, parameters, block.timestamp)
- **"it should revert"** - Revert outcome
- **"it should succeed"** - Happy path outcome

## Example

### Input: Solidity Contract

```solidity
contract Vault {
    address public owner;
    uint256 public balance;

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    function withdraw(uint256 amount) external onlyOwner {
        require(amount > 0, "Amount must be positive");
        require(balance >= amount, "Insufficient balance");
        balance -= amount;
    }
}
```

### Output: Test Tree

```
withdraw
├── given msg.sender is not owner
│   └── it should revert
└── given msg.sender is owner
    ├── when amount is at most zero
    │   └── it should revert
    └── when amount is greater than zero
        ├── given balance is less than amount
        │   └── it should revert
        └── given balance is at least amount
            └── it should succeed
```

## CI/CD Integration

```yaml
# .github/workflows/test-trees.yml
- name: Install Acacia
  run: cargo install acacia

- name: Generate Test Trees
  run: acacia generate MyContract::withdraw
```

## Requirements

- **Foundry Project** with `foundry.toml`

## Limitations

- Does not trace into external contract calls (by design)
- Function overloading not yet supported

## License

MIT

---

*Built with [Solar](https://github.com/paradigmxyz/solar) - the Solidity compiler in Rust*
