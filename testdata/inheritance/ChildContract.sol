// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./Ownable.sol";

/// @title Child contract that inherits from Ownable
contract ChildContract is Ownable {
    uint256 public value;

    /// @notice Only owner can set value, and value must be positive
    function setValue(uint256 newValue) external onlyOwner {
        require(newValue > 0, "Value must be positive");
        value = newValue;
    }
}
