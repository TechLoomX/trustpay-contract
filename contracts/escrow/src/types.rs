use soroban_sdk::{contracttype, Address, BytesN, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Active,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending,   // created, not yet funded
    Funded,    // client has deposited
    Submitted, // freelancer marked as done
    Approved,  // client approved — triggers release
    Released,  // funds have moved to freelancer
    Disputed,  // frozen pending resolution
    Refunded,  // returned to client
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub index: u32,
    pub amount: i128,
    pub description_hash: BytesN<32>, // hash of off-chain description; keeps on-chain storage cheap
    pub status: MilestoneStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub id: u64,
    pub client: Address,
    pub freelancer: Address,
    pub token: Address,
    pub milestones: Vec<Milestone>,
    pub status: EscrowStatus,
    pub created_at: u64,
}

#[contracttype]
pub enum DataKey {
    Escrow(u64), // escrow_id -> Escrow
    NextEscrowId, // counter
}
