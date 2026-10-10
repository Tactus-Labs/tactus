// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// @notice Native CKB accounting and permanent withdrawal commitments.
/// @dev Zero-sender credits require a NEW, L1-authenticated execution profile.
/// The existing v1 Executor cannot mint. This contract alone authorizes no L1 release.
contract NativeCKB {
    string public constant name = "Wrapped CKB";
    string public constant symbol = "wCKB";
    uint8 public constant decimals = 8;
    bytes32 public immutable DOMAIN;

    // Storage positions are part of the CKB withdrawal verifier's wire contract.
    uint256 public totalSupply;                          // slot 0
    uint256 public cumulativeDeposited;                  // slot 1
    uint256 public cumulativeWithdrawn;                  // slot 2
    mapping(address => uint256) public balanceOf;        // slot 3
    mapping(address => mapping(address => uint256)) public allowance; // slot 4
    mapping(bytes32 => bool) public creditedDeposits;     // slot 5
    uint64 public withdrawalCount;                       // slot 6
    mapping(uint64 => bytes32) public withdrawals;        // slot 7

    error UnauthorizedCredit();
    error InvalidDeposit();
    error DuplicateDeposit();
    error InvalidRecipient();
    error InsufficientBalance();
    error InsufficientAllowance();
    error InvalidWithdrawal();
    error SupplyLimit();

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event DepositCredited(bytes32 indexed depositId, address indexed recipient, uint64 amount);
    event WithdrawalRequested(uint64 indexed id, address indexed owner, bytes32 indexed recipientLockHash,
                              uint64 amount, bytes32 commitment);

    /// @param bridgeDomain Commitment to CKB genesis, rollup and the unique vault identity.
    /// Deployment is ordinary EVM execution; no deployer receives mint authority.
    constructor(bytes32 bridgeDomain) {
        if (bridgeDomain == bytes32(0)) revert InvalidDeposit();
        DOMAIN = keccak256(abi.encodePacked(bytes8("TO1BRDG1"), bridgeDomain, block.chainid, address(this)));
    }

    /// @dev Only the authenticated deposit execution hook may use sender zero.
    /// Admission/authentication/once-only L1 consumption are required outside this contract.
    function creditDeposit(bytes32 depositId, address recipient, uint64 amount) external {
        if (msg.sender != address(0)) revert UnauthorizedCredit();
        if (depositId == bytes32(0) || amount == 0) revert InvalidDeposit();
        if (recipient == address(0) || recipient == address(this)) revert InvalidRecipient();
        if (creditedDeposits[depositId]) revert DuplicateDeposit();
        if (totalSupply + amount > type(uint64).max) revert SupplyLimit();
        creditedDeposits[depositId] = true;
        cumulativeDeposited += amount;
        totalSupply += amount;
        balanceOf[recipient] += amount;
        emit Transfer(address(0), recipient, amount);
        emit DepositCredited(depositId, recipient, amount);
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transfer(address recipient, uint256 amount) external returns (bool) {
        _transfer(msg.sender, recipient, amount);
        return true;
    }

    function transferFrom(address owner, address recipient, uint256 amount) external returns (bool) {
        uint256 permitted = allowance[owner][msg.sender];
        if (permitted < amount) revert InsufficientAllowance();
        if (permitted != type(uint256).max) allowance[owner][msg.sender] = permitted - amount;
        _transfer(owner, recipient, amount);
        return true;
    }

    function _transfer(address owner, address recipient, uint256 amount) private {
        if (recipient == address(0) || recipient == address(this)) revert InvalidRecipient();
        if (balanceOf[owner] < amount) revert InsufficientBalance();
        balanceOf[owner] -= amount;
        balanceOf[recipient] += amount;
        emit Transfer(owner, recipient, amount);
    }

    /// @notice Destroy tokens and commit an exact CKB recipient lock hash and amount.
    /// CKB fees are funded separately; the committed amount may not be reduced by a relayer.
    /// Claims persist forever. Replay prevention belongs to the unique CKB vault state.
    function withdraw(uint64 amount, bytes32 recipientLockHash) external returns (uint64 id) {
        if (amount == 0 || recipientLockHash == bytes32(0)) revert InvalidWithdrawal();
        if (balanceOf[msg.sender] < amount) revert InsufficientBalance();
        balanceOf[msg.sender] -= amount;
        totalSupply -= amount;
        cumulativeWithdrawn += amount;
        id = ++withdrawalCount;
        bytes32 commitment = keccak256(abi.encodePacked(
            bytes8("TO1EXIT1"), DOMAIN, id, amount, recipientLockHash, msg.sender
        ));
        withdrawals[id] = commitment;
        emit Transfer(msg.sender, address(0), amount);
        emit WithdrawalRequested(id, msg.sender, recipientLockHash, amount, commitment);
    }
}
