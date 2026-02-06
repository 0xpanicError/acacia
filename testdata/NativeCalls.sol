
    contract TestNative {
        function testEthTransfer(address payable recipient, uint256 amount) public {
            recipient.transfer(amount);
        }
    }
    