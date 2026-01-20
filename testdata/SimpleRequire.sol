// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Simple contract with a single require statement
contract SimpleRequire {
    function transfer(address to, uint256 amount) external {
        require(amount > 0, "Amount must be positive");
        // transfer logic
    }
}
