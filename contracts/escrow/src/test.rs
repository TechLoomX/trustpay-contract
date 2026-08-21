#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Env,
};

fn create_token<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(env, &sac.address()),
        StellarAssetClient::new(env, &sac.address()),
    )
}

struct Setup<'a> {
    env: Env,
    client: Address,
    freelancer: Address,
    token: TokenClient<'a>,
    _token_admin: StellarAssetClient<'a>,
    contract: EscrowContractClient<'a>,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token, token_admin_client) = create_token(&env, &token_admin);
    token_admin_client.mint(&client, &1_000_000);

    let contract_id = env.register(EscrowContract, ());
    let contract = EscrowContractClient::new(&env, &contract_id);

    Setup {
        env,
        client,
        freelancer,
        token,
        _token_admin: token_admin_client,
        contract,
    }
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn test_happy_path() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);

    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    s.contract.deposit(&escrow_id, &0);
    assert_eq!(s.token.balance(&s.client), 1_000_000 - 100);
    assert_eq!(s.token.balance(&s.contract.address), 100);

    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.approve_milestone(&escrow_id, &0);

    assert_eq!(s.token.balance(&s.freelancer), 100);
    assert_eq!(s.token.balance(&s.client), 1_000_000 - 100);

    let milestone = s.contract.get_milestone(&escrow_id, &0);
    assert_eq!(milestone.status, MilestoneStatus::Released);

    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Completed);
}

#[test]
fn test_multi_milestone_completion() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128, 200i128, 300i128]);
    let hashes = Vec::from_array(env, [hash(env, 1), hash(env, 2), hash(env, 3)]);

    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    for i in 0..3u32 {
        s.contract.deposit(&escrow_id, &i);
        s.contract.submit_milestone(&escrow_id, &i);
        s.contract.approve_milestone(&escrow_id, &i);

        let escrow = s.contract.get_escrow(&escrow_id);
        if i < 2 {
            assert_eq!(escrow.status, EscrowStatus::Active);
        } else {
            assert_eq!(escrow.status, EscrowStatus::Completed);
        }
    }

    assert_eq!(s.token.balance(&s.freelancer), 600);
}

#[test]
fn test_refund_path() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    s.contract.deposit(&escrow_id, &0);
    assert_eq!(s.token.balance(&s.client), 1_000_000 - 100);

    s.contract.refund(&escrow_id, &0);
    assert_eq!(s.token.balance(&s.client), 1_000_000);

    let milestone = s.contract.get_milestone(&escrow_id, &0);
    assert_eq!(milestone.status, MilestoneStatus::Refunded);
}

// require_auth() is enforced by the host itself: an invocation missing a
// matching auth entry traps rather than returning a contract-level error, so
// these are asserted via should_panic rather than an Error::NotAuthorized check.

#[test]
#[should_panic]
fn test_submit_by_non_freelancer_panics() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );
    s.contract.deposit(&escrow_id, &0);

    env.set_auths(&[]);
    s.contract.submit_milestone(&escrow_id, &0);
}

#[test]
#[should_panic]
fn test_deposit_by_non_client_panics() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    env.set_auths(&[]);
    s.contract.deposit(&escrow_id, &0);
}

#[test]
fn test_dispute_by_stranger_returns_not_authorized() {
    let s = setup();
    let env = &s.env;
    let stranger = Address::generate(env);

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );
    s.contract.deposit(&escrow_id, &0);

    let res = s
        .contract
        .try_raise_dispute(&escrow_id, &0, &stranger)
        .unwrap_err()
        .unwrap();
    assert_eq!(res, Error::NotAuthorized);
}

#[test]
fn test_invalid_transitions() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    // approve without submit
    let res = s
        .contract
        .try_approve_milestone(&escrow_id, &0)
        .unwrap_err()
        .unwrap();
    assert_eq!(res, Error::InvalidStatus);

    // submit without deposit
    let res = s
        .contract
        .try_submit_milestone(&escrow_id, &0)
        .unwrap_err()
        .unwrap();
    assert_eq!(res, Error::InvalidStatus);

    s.contract.deposit(&escrow_id, &0);

    // deposit twice
    let res = s.contract.try_deposit(&escrow_id, &0).unwrap_err().unwrap();
    assert_eq!(res, Error::InvalidStatus);
}

#[test]
fn test_dispute_freeze() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    s.contract.deposit(&escrow_id, &0);
    s.contract.submit_milestone(&escrow_id, &0);
    s.contract.raise_dispute(&escrow_id, &0, &s.client);

    let milestone = s.contract.get_milestone(&escrow_id, &0);
    assert_eq!(milestone.status, MilestoneStatus::Disputed);

    let res = s
        .contract
        .try_approve_milestone(&escrow_id, &0)
        .unwrap_err()
        .unwrap();
    assert_eq!(res, Error::InvalidStatus);

    let res = s.contract.try_refund(&escrow_id, &0).unwrap_err().unwrap();
    assert_eq!(res, Error::InvalidStatus);
}

#[test]
fn test_cancel_with_mixed_milestone_states() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128, 200i128]);
    let hashes = Vec::from_array(env, [hash(env, 1), hash(env, 2)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    // milestone 0 stays Pending, milestone 1 gets funded.
    s.contract.deposit(&escrow_id, &1);
    assert_eq!(s.token.balance(&s.client), 1_000_000 - 200);

    s.contract.cancel_escrow(&escrow_id);

    assert_eq!(s.token.balance(&s.client), 1_000_000);
    let escrow = s.contract.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
    assert_eq!(
        escrow.milestones.get(1).unwrap().status,
        MilestoneStatus::Refunded
    );
}

#[test]
fn test_cancel_fails_if_submitted() {
    let s = setup();
    let env = &s.env;

    let amounts = Vec::from_array(env, [100i128]);
    let hashes = Vec::from_array(env, [hash(env, 1)]);
    let escrow_id = s.contract.create_escrow(
        &s.client,
        &s.freelancer,
        &s.token.address,
        &amounts,
        &hashes,
    );

    s.contract.deposit(&escrow_id, &0);
    s.contract.submit_milestone(&escrow_id, &0);

    let res = s
        .contract
        .try_cancel_escrow(&escrow_id)
        .unwrap_err()
        .unwrap();
    assert_eq!(res, Error::InvalidStatus);
}
