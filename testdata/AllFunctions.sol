// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title Contract with mixed visibility functions for testing
contract AllFunctions {
    bool public paused;
    address public owner;

    /// @notice External function - should be included
    function externalFunc(uint256 amount) external pure {
        require(amount > 0, "Invalid amount");
    }

    /// @notice Public function - should be included
    function publicFunc(address to) public pure {
        require(to != address(0), "Invalid address");
    }

    /// @notice Internal function - should be excluded
    function internalFunc(uint256 x) internal pure returns (uint256) {
        require(x > 0, "Invalid");
        return x * 2;
    }

    /// @notice Private function - should be excluded
    function privateFunc() private pure returns (bool) {
        return true;
    }

    /// @notice Another public function with modifier pattern
    function pausableFunc() public view {
        require(!paused, "Contract is paused");
    }
}
