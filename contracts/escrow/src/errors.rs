use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    EscrowNotFound = 1,
    MilestoneNotFound = 2,
    NotAuthorized = 3,
    InvalidStatus = 4,
    InvalidAmount = 5,
    MismatchedLengths = 6,
    EscrowNotActive = 7,
}
