
    contract TestContract {
        function testSendValue(address payable recipient, uint256 amount) public {
            Address.sendValue(recipient, amount);
        }
        
        function testFunctionCall(address target, bytes memory data) public {
            Address.functionCall(target, data);
        }
    }
    