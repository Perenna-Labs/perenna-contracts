use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StreamError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    StreamNotFound = 4,
    /// Reserved for ABI stability; unused in current contract logic.
    StreamEnded = 5,
    StreamCanceled = 6,
    InvalidTimeRange = 7,
    ZeroDeposit = 8,
    /// Reserved for ABI stability; unused in current contract logic.
    AmountMismatch = 9,
    WithdrawAmountTooHigh = 10,
    NotCancelable = 11,
    MathOverflow = 12,
    InvalidAdmin = 13,
    InvalidWithdrawAmount = 13,
}
