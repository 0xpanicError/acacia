// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Contract with storage-based condition
contract StorageCondition {
    bool public paused;
    address public owner;
    
    function setPaused(bool _paused) external {
        require(msg.sender == owner, "Not owner");
        paused = _paused;
    }
    
    function doSomething() external {
        require(!paused, "Contract is paused");
        // logic
    }
}
