// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title Base contract with ownership modifier
abstract contract Ownable {
    address public owner;

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
}
