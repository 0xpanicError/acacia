// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title Contract with overloaded functions for testing
contract FunctionOverloading {
    /// @notice Transfer to an address (1 parameter)
    function transfer(address to) external pure {
        require(to != address(0), "Invalid address");
    }

    /// @notice Transfer to an address with amount (2 parameters)
    function transfer(address to, uint256 amount) external pure {
        require(to != address(0), "Invalid address");
        require(amount > 0, "Invalid amount");
    }

    /// @notice Transfer to an address with amount and data (3 parameters)
    function transfer(address to, uint256 amount, bytes calldata data) external pure {
        require(to != address(0), "Invalid address");
        require(amount > 0, "Invalid amount");
        require(data.length > 0, "Invalid data");
    }
}
