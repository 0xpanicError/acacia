// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Contract with a modifier
contract WithModifier {
    address public owner;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function mint(uint256 amount) external onlyOwner {
        require(amount > 0, "Amount must be positive");
        // mint logic
    }
}
