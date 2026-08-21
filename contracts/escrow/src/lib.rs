#![no_std]

mod errors;
mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};

pub use errors::Error;
use events::{
    DisputeRaised, EscrowCancelled, EscrowCreated, FundsReleased, MilestoneApproved,
    MilestoneFunded, MilestoneRefunded, MilestoneSubmitted,
};
pub use types::{DataKey, Escrow, EscrowStatus, Milestone, MilestoneStatus};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn create_escrow(
        env: Env,
        client: Address,
        freelancer: Address,
        token: Address,
        milestone_amounts: Vec<i128>,
        milestone_hashes: Vec<BytesN<32>>,
    ) -> Result<u64, Error> {
        client.require_auth();

        if milestone_amounts.is_empty() || milestone_amounts.len() != milestone_hashes.len() {
            return Err(Error::MismatchedLengths);
        }
        for amount in milestone_amounts.iter() {
            if amount <= 0 {
                return Err(Error::InvalidAmount);
            }
        }

        let escrow_id = Self::next_escrow_id(&env);

        let mut milestones = Vec::new(&env);
        for i in 0..milestone_amounts.len() {
            milestones.push_back(Milestone {
                index: i,
                amount: milestone_amounts.get(i).unwrap(),
                description_hash: milestone_hashes.get(i).unwrap(),
                status: MilestoneStatus::Pending,
            });
        }

        let escrow = Escrow {
            id: escrow_id,
            client: client.clone(),
            freelancer: freelancer.clone(),
            token,
            milestones,
            status: EscrowStatus::Active,
            created_at: env.ledger().timestamp(),
        };

        Self::save_escrow(&env, &escrow);

        EscrowCreated {
            escrow_id,
            client,
            freelancer,
        }
        .publish(&env);

        Ok(escrow_id)
    }

    pub fn deposit(env: Env, escrow_id: u64, milestone_index: u32) -> Result<(), Error> {
        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        escrow.client.require_auth();

        if escrow.status != EscrowStatus::Active {
            return Err(Error::EscrowNotActive);
        }

        let mut milestone = Self::milestone_at(&escrow, milestone_index)?;
        if milestone.status != MilestoneStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &escrow.client,
            env.current_contract_address(),
            &milestone.amount,
        );

        milestone.status = MilestoneStatus::Funded;
        let amount = milestone.amount;
        escrow.milestones.set(milestone_index, milestone);
        Self::save_escrow(&env, &escrow);

        MilestoneFunded {
            escrow_id,
            milestone_index,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn submit_milestone(env: Env, escrow_id: u64, milestone_index: u32) -> Result<(), Error> {
        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        escrow.freelancer.require_auth();

        let mut milestone = Self::milestone_at(&escrow, milestone_index)?;
        if milestone.status != MilestoneStatus::Funded {
            return Err(Error::InvalidStatus);
        }

        milestone.status = MilestoneStatus::Submitted;
        escrow.milestones.set(milestone_index, milestone);
        Self::save_escrow(&env, &escrow);

        MilestoneSubmitted {
            escrow_id,
            milestone_index,
        }
        .publish(&env);

        Ok(())
    }

    pub fn approve_milestone(env: Env, escrow_id: u64, milestone_index: u32) -> Result<(), Error> {
        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        escrow.client.require_auth();

        let mut milestone = Self::milestone_at(&escrow, milestone_index)?;
        if milestone.status != MilestoneStatus::Submitted {
            return Err(Error::InvalidStatus);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &milestone.amount,
        );

        milestone.status = MilestoneStatus::Released;
        let amount = milestone.amount;
        escrow.milestones.set(milestone_index, milestone);

        if escrow.milestones.iter().all(|m| {
            matches!(
                m.status,
                MilestoneStatus::Released | MilestoneStatus::Refunded
            )
        }) {
            escrow.status = EscrowStatus::Completed;
        }

        Self::save_escrow(&env, &escrow);

        MilestoneApproved {
            escrow_id,
            milestone_index,
        }
        .publish(&env);
        FundsReleased {
            escrow_id,
            milestone_index,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn raise_dispute(
        env: Env,
        escrow_id: u64,
        milestone_index: u32,
        caller: Address,
    ) -> Result<(), Error> {
        caller.require_auth();

        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        if caller != escrow.client && caller != escrow.freelancer {
            return Err(Error::NotAuthorized);
        }

        let mut milestone = Self::milestone_at(&escrow, milestone_index)?;
        if !matches!(
            milestone.status,
            MilestoneStatus::Funded | MilestoneStatus::Submitted
        ) {
            return Err(Error::InvalidStatus);
        }

        milestone.status = MilestoneStatus::Disputed;
        escrow.milestones.set(milestone_index, milestone);
        Self::save_escrow(&env, &escrow);

        DisputeRaised {
            escrow_id,
            milestone_index,
            caller,
        }
        .publish(&env);

        Ok(())
    }

    pub fn refund(env: Env, escrow_id: u64, milestone_index: u32) -> Result<(), Error> {
        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        escrow.client.require_auth();

        let mut milestone = Self::milestone_at(&escrow, milestone_index)?;
        if milestone.status != MilestoneStatus::Funded {
            return Err(Error::InvalidStatus);
        }

        let token_client = token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.client,
            &milestone.amount,
        );

        milestone.status = MilestoneStatus::Refunded;
        escrow.milestones.set(milestone_index, milestone);
        Self::save_escrow(&env, &escrow);

        MilestoneRefunded {
            escrow_id,
            milestone_index,
        }
        .publish(&env);

        Ok(())
    }

    pub fn cancel_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        let mut escrow = Self::load_escrow(&env, escrow_id)?;
        escrow.client.require_auth();

        for i in 0..escrow.milestones.len() {
            let m = escrow.milestones.get(i).unwrap();
            if matches!(
                m.status,
                MilestoneStatus::Submitted | MilestoneStatus::Approved | MilestoneStatus::Disputed
            ) {
                return Err(Error::InvalidStatus);
            }
        }

        let token_client = token::Client::new(&env, &escrow.token);
        for i in 0..escrow.milestones.len() {
            let mut m = escrow.milestones.get(i).unwrap();
            if m.status == MilestoneStatus::Funded {
                token_client.transfer(&env.current_contract_address(), &escrow.client, &m.amount);
                m.status = MilestoneStatus::Refunded;
                escrow.milestones.set(i, m);
            }
        }

        escrow.status = EscrowStatus::Cancelled;
        Self::save_escrow(&env, &escrow);

        EscrowCancelled { escrow_id }.publish(&env);

        Ok(())
    }

    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, Error> {
        Self::load_escrow(&env, escrow_id)
    }

    pub fn get_milestone(
        env: Env,
        escrow_id: u64,
        milestone_index: u32,
    ) -> Result<Milestone, Error> {
        let escrow = Self::load_escrow(&env, escrow_id)?;
        Self::milestone_at(&escrow, milestone_index)
    }

    fn next_escrow_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextEscrowId)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::NextEscrowId, &(id + 1));
        id
    }

    fn load_escrow(env: &Env, escrow_id: u64) -> Result<Escrow, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)
    }

    fn save_escrow(env: &Env, escrow: &Escrow) {
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow.id), escrow);
    }

    fn milestone_at(escrow: &Escrow, milestone_index: u32) -> Result<Milestone, Error> {
        escrow
            .milestones
            .get(milestone_index)
            .ok_or(Error::MilestoneNotFound)
    }
}
