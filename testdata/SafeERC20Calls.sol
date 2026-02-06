
    contract TestToken {
        using SafeERC20 for IERC20;
        IERC20 token;
        
        function testSafeTransfer(address to, uint256 amount) public {
            token.safeTransfer(to, amount);
        }
        
        function testTransfer(address to, uint256 amount) public {
            token.transfer(to, amount);
        }
    }
    