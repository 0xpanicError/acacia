// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Contract with if-revert pattern
contract IfRevert {
    uint256 public maxSupply = 1000;
    uint256 public totalSupply;
    
    error MaxSupplyReached();
    error InvalidAmount();
    
    function mint(uint256 amount) external {
        if (amount == 0) {
            revert InvalidAmount();
        }
        if (totalSupply + amount > maxSupply) {
            revert MaxSupplyReached();
        }
        totalSupply += amount;
    }
}
