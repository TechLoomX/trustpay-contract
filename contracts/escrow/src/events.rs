use soroban_sdk::{contractevent, Address};

#[contractevent]
pub struct EscrowCreated {
    #[topic]
    pub escrow_id: u64,
    pub client: Address,
    pub freelancer: Address,
}

#[contractevent]
pub struct MilestoneFunded {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
    pub amount: i128,
}

#[contractevent]
pub struct MilestoneSubmitted {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
}

#[contractevent]
pub struct MilestoneApproved {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
}

#[contractevent]
pub struct FundsReleased {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
    pub amount: i128,
}

#[contractevent]
pub struct DisputeRaised {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
    pub caller: Address,
}

#[contractevent]
pub struct MilestoneRefunded {
    #[topic]
    pub escrow_id: u64,
    #[topic]
    pub milestone_index: u32,
}

#[contractevent]
pub struct EscrowCancelled {
    #[topic]
    pub escrow_id: u64,
}
