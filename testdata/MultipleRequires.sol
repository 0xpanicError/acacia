// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// Contract with multiple sequential require statements
contract MultipleRequires {
    mapping(address => uint256) public balances;
    
    function withdraw(uint256 amount) external {
        require(amount > 0, "Amount must be positive");
        require(balances[msg.sender] >= amount, "Insufficient balance");
        require(address(this).balance >= amount, "Contract insufficient funds");
        balances[msg.sender] -= amount;
    }
}
