// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Contract with loop containing require
contract WithLoop {
    function batchTransfer(address[] calldata recipients, uint256[] calldata amounts) external {
        require(recipients.length == amounts.length, "Length mismatch");
        for (uint256 i = 0; i < recipients.length; i++) {
            require(recipients[i] != address(0), "Invalid recipient");
            require(amounts[i] > 0, "Invalid amount");
            // transfer logic
        }
    }
}
