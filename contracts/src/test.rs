#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::token;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, testutils::Address as _, testutils::Events,
    vec, Address, Env, IntoVal, String, Vec,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
fn test_add_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    env.mock_all_auths();
    let member1 = Address::generate(&env);
    client.add_member(&member1);
}

#[test]
#[should_panic(expected = "Not a member")]
fn test_contribute_not_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    env.mock_all_auths();
    let not_member = Address::generate(&env);
    client.contribute(&not_member, &1000);
}

#[test]
#[should_panic(expected = "Already contributed this cycle")]
fn test_contribute_twice_same_cycle_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.contribute(&member, &1000);
}

#[test]
fn test_contribute_allowed_after_reset() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.reset_cycle();
    client.contribute(&member, &1000);

    assert_eq!(client.get_contribution(&member), 2000);
}

#[test]
fn test_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    // 1. Test Initialize Event
    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                soroban_sdk::vec![&env, symbol_short!("init").into_val(&env)],
                (
                    admin.clone(),
                    token.clone(),
                    name.clone(),
                    contribution_amount
                )
                    .into_val(&env)
            )
        ]
    );

    // 2. Test Add Member Event
    let member1 = Address::generate(&env);
    client.add_member(&member1);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                soroban_sdk::vec![
                    &env,
                    symbol_short!("add_mem").into_val(&env),
                    member1.clone().into_val(&env)
                ],
                ().into_val(&env)
            )
        ]
    );
}

#[test]
fn test_goalbased_flexible_contributions() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    // Can contribute varying amounts
    client.contribute(&member, &500);

    // Wait, the test above calls contribute twice in a row, but HasContributedThisCycle is still active.
    // So we need to call reset_cycle() or it will panic.
    client.reset_cycle();
    client.contribute(&member, &1500);

    assert_eq!(client.get_contribution(&member), 2000);
}

#[test]
fn test_goalbased_partial_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &2000);

    assert_eq!(token_client.balance(&member), 3000);

    client.withdraw_savings(&member, &500);

    assert_eq!(client.get_contribution(&member), 1500);
    assert_eq!(token_client.balance(&member), 3500);
}

#[test]
#[should_panic(expected = "Insufficient savings to withdraw")]
fn test_goalbased_over_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.withdraw_savings(&member, &1500);
}

#[test]
#[should_panic(expected = "Target amount not reached yet")]
fn test_goalbased_locked_until_target() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &Some(2000), // target amount
        &true,       // lock until target
        &None,       // expected_cycle_days
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    // Should panic because 1000 < 2000
    client.withdraw_savings(&member, &500);
}

#[test]
fn test_remove_member_no_contribution() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);

    client.remove_member(&member2);

    env.as_contract(&contract_id, || {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.contains(&member1));
        assert!(!members.contains(&member2));
    });
}

#[test]
fn test_remove_member_with_contribution_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);

    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(token_client.balance(&member2), 4000);

    client.remove_member(&member2);

    assert_eq!(token_client.balance(&member2), 5000);

    env.as_contract(&contract_id, || {
        // MemberState entry removed entirely during remove_member (O(1) design)
        assert!(!env
            .storage()
            .instance()
            .has(&DataKey::Member(member2.clone())));
    });
}

#[test]
#[should_panic(expected = "Cannot remove member after their payout turn")]
fn test_remove_member_after_payout_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);

    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    client.payout(&member1, &None);

    client.remove_member(&member1);
}

#[test]
fn test_remove_member_adjusts_cycle_count() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    client.add_member(&member3);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);
    token_client.mint(&member3, &5000);

    client.contribute(&member1, &1000);

    env.as_contract(&contract_id, || {
        let count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count, 3);
    });

    client.remove_member(&member3);

    env.as_contract(&contract_id, || {
        let count_after: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count_after, 2);
    });

    client.contribute(&member2, &1000);

    client.payout(&member1, &None);

    let contract_balance = token_client.balance(&contract_id);
    assert_eq!(contract_balance, 0);

    client.reset_cycle();

    env.as_contract(&contract_id, || {
        assert!(!env.storage().instance().has(&DataKey::CycleMemberCount));
    });
}

#[test]
fn test_remove_last_member_clears_cycle_count() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);

    env.as_contract(&contract_id, || {
        let count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count, 1);
    });

    client.remove_member(&member);

    env.as_contract(&contract_id, || {
        assert!(!env.storage().instance().has(&DataKey::CycleMemberCount));
    });
}

#[test]
fn test_deterministic_payout_order() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // Payout 1 goes to member0 (index 0 in join order)
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout(&member0, &None);
    assert!(client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));
    // member0 gets the full pool: 1000 * 3 = 3000
    // balance = 10000 - 1000 (contrib) + 3000 (payout) = 12000
    assert_eq!(token_client.balance(&member0), 12000);

    // After payout, NextPayoutIndex advanced to 1 — member1 is next
    assert_eq!(client.get_next_payout_recipient(), member1);

    // Payout 2 goes to member1 (index 1) after reset + re-contribute
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout(&member1, &None);
    assert!(client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));
    // 10000 - 1000 (round1) - 1000 (round2) + 3000 (payout) = 11000
    assert_eq!(token_client.balance(&member1), 11000);

    // Payout 3 goes to member2 (index 2)
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout(&member2, &None);
    assert!(client.has_received_payout(&member2));
    // 10000 - 1000*3 (3 rounds) + 3000 (payout) = 10000
    assert_eq!(token_client.balance(&member2), 10000);
}

#[test]
fn test_queue_enforced_payout_order() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // Payout must go to member0 (index 0), admin cannot choose
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout(&member0, &None);
    assert!(client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member2));

    // After reset, queue advances to member1 (NextPayoutIndex persists)
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout(&member1, &None);
    assert!(client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));

    // After another reset, finally member2
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout(&member2, &None);
    assert!(client.has_received_payout(&member2));
}

#[test]
fn test_cycle_resets_and_starts_again() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // --- Full rotation: member0 → member1 → member2 ---
    // Round 1: payout to member0
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout(&member0, &None);
    assert!(client.has_received_payout(&member0));

    // Round 2: payout to member1
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout(&member1, &None);
    assert!(client.has_received_payout(&member1));

    // Round 3: payout to member2
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout(&member2, &None);
    assert!(client.has_received_payout(&member2));

    // --- Full reset: reset_rotation() resets NextPayoutIndex to 0 ---
    client.reset_cycle();
    client.reset_rotation();

    // After full reset, all has_received_payout flags are cleared
    assert!(!client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));

    // New rotation starts with member0 again
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout(&member0, &None);
    assert!(client.has_received_payout(&member0));
}

#[test]
#[should_panic(expected = "Contribution amount must be positive")]
fn test_initialize_with_negative_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &-1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Contribution amount must be positive")]
fn test_initialize_with_zero_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &0i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Contribution amount exceeds maximum limit")]
fn test_initialize_with_exceeding_max_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &1_000_000_000_000_001i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Math overflow in pool calculation")]
fn test_payout_pool_size_overflow_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    client.add_member(&member0);

    env.as_contract(&contract_id, || {
        // Force an overflow scenario
        env.storage()
            .instance()
            .set(&DataKey::ContributionAmount, &i128::MAX);
        env.storage()
            .instance()
            .set(&DataKey::CycleMemberCount, &2i128);
    });

    client.payout(&member0, &None);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_payout_wrong_recipient_auth_fails() {
    let env = Env::default();

    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    // Initialize with mock all auths to easily bypass initialization auth
    env.mock_all_auths();
    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);

    token_client.mint(&member0, &10000);
    client.contribute(&member0, &1000);

    // We only explicitly mock the auth for payout with the WRONG recipient (member1)
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "payout",
            args: (member1.clone(), None::<i128>).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // Calling it with member0 should fail because auth is for member1
    client.payout(&member0, &None);
}

// ---------------------------------------------------------------------------
// Partial payout tests (proceedWithPartialPool support)
// ---------------------------------------------------------------------------

#[test]
fn test_payout_partial_amount_succeeds_and_advances_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);

    // Only member0 and member1 contribute this cycle — member2 defaults.
    // The full pool would be 1000 * 3 = 3000, but the contract only holds
    // 2000, so the admin proceeds with a partial payout of exactly that.
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);

    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout(&member0, &Some(2000i128));

    assert!(client.has_received_payout(&member0));
    // 10000 - 1000 (contrib) + 2000 (partial payout) = 11000
    assert_eq!(token_client.balance(&member0), 11000);
    assert_eq!(token_client.balance(&contract_id), 0);

    // Rotation still advances normally after a partial payout.
    assert_eq!(client.get_next_payout_recipient(), member1);
}

#[test]
#[should_panic(expected = "Payout amount exceeds the full computed pool")]
fn test_payout_partial_amount_above_full_pool_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);

    // Full pool is 1000 * 2 = 2000 — asking for more than that is rejected
    // even though the admin is authorizing it, so a compromised or buggy
    // caller can never drain more than a cycle's own pool.
    client.payout(&member0, &Some(2001i128));
}

#[test]
#[should_panic(expected = "Payout amount must be positive")]
fn test_payout_partial_amount_zero_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    client.add_member(&member0);
    token_client.mint(&member0, &10000);
    client.contribute(&member0, &1000);

    client.payout(&member0, &Some(0i128));
}

#[test]
#[should_panic(expected = "Insufficient funds in contract for requested payout")]
fn test_payout_partial_amount_above_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    // Only member0 contributes — contract balance is 1000, but the full pool
    // (1000 * 3 = 3000) is still within bounds for an explicit request, so
    // this must fail on the balance check, not the pool-size ceiling check.
    client.contribute(&member0, &1000);

    client.payout(&member0, &Some(1500i128));
}

// ---------------------------------------------------------------------------
// Reentrancy / Checks-Effects-Interactions tests
// ---------------------------------------------------------------------------
//
// `MaliciousToken` is a minimal mock that implements just enough of the token
// interface (`balance` and `transfer`) for `token::Client` to invoke it from
// inside `KoloSavingsContract`. Its `transfer` function is where a real
// malicious or unusually-implemented token could try to "call back" into the
// savings contract mid-payout. We use it to:
//
//   1. Prove the CEI ordering: by the time `transfer` runs, the effects of
//      `payout()` (NextPayoutIndex / has_received_payout) are already
//      committed — observable via a read-only call made from inside the
//      token's `transfer` callback.
//   2. Prove the reentrancy guard: a callback that tries to invoke `payout()`
//      again while the outer `payout()` call is still executing must panic
//      with "Reentrancy detected", rather than draining the pool twice.
//
// Note: because Soroban contract invocations are atomic, a panic anywhere
// during a call unwinds and reverts *all* storage writes made during that
// top-level call. This means we can't inspect "partial" state after a failed
// transfer from outside — the whole test would revert. So the meaningful way
// to verify ordering is to observe state *during* the call, before it either
// completes or panics, which is exactly what these mocks do.

#[contracttype]
#[derive(Clone)]
enum MalKey {
    Kolo,
    Mode,
    ObservedHasPayout,
}

#[contract]
pub struct MaliciousToken;

#[contractimpl]
impl MaliciousToken {
    /// Configure which savings contract to target. Starts in "observe" mode (0).
    pub fn init(env: Env, kolo: Address) {
        env.storage().instance().set(&MalKey::Kolo, &kolo);
        env.storage().instance().set(&MalKey::Mode, &0u32);
    }

    /// 0 = passively observe state during transfer(); 1 = attempt a reentrant
    /// call into payout() during transfer().
    pub fn set_mode(env: Env, mode: u32) {
        env.storage().instance().set(&MalKey::Mode, &mode);
    }

    pub fn get_observed(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&MalKey::ObservedHasPayout)
            .unwrap_or(false)
    }

    pub fn balance(env: Env, _id: Address) -> i128 {
        let mode: u32 = env.storage().instance().get(&MalKey::Mode).unwrap_or(0);
        if mode == 2 {
            let kolo: Address = env.storage().instance().get(&MalKey::Kolo).unwrap();
            let client = KoloSavingsContractClient::new(&env, &kolo);
            // Attempt reentrancy from inside balance(), before transfer() is ever reached.
            client.payout(&kolo, &None); // recipient doesn't matter — should never get this far
        }
        i128::MAX
    }

    /// Called by KoloSavingsContract via `token::Client::transfer`. This is the
    /// "interaction" step where a malicious token gets a chance to re-enter.
    pub fn transfer(env: Env, _from: Address, to: Address, _amount: i128) {
        let kolo: Address = env.storage().instance().get(&MalKey::Kolo).unwrap();
        let mode: u32 = env.storage().instance().get(&MalKey::Mode).unwrap_or(0);

        if mode == 1 {
            // Attack: try to trigger a second payout to the same recipient
            // while the outer payout() call is still mid-flight.
            let client = KoloSavingsContractClient::new(&env, &kolo);
            client.payout(&to, &None);
        } else {
            // Observe: read back whether payout()'s effects were already
            // applied by the time this callback runs.
            let already_marked: bool = env.as_contract(&kolo, || {
                let state: Option<MemberState> =
                    env.storage().persistent().get(&DataKey::Member(to.clone()));
                state.map(|s| s.has_received_payout).unwrap_or(false)
            });
            env.storage()
                .instance()
                .set(&MalKey::ObservedHasPayout, &already_marked);
        }
    }
}

fn setup_with_malicious_token(
    env: &Env,
) -> (
    Address,
    KoloSavingsContractClient<'_>,
    Address,
    MaliciousTokenClient<'_>,
) {
    let kolo_id = env.register_contract(None, KoloSavingsContract);
    let kolo_client = KoloSavingsContractClient::new(env, &kolo_id);

    let mal_token_id = env.register_contract(None, MaliciousToken);
    let mal_client = MaliciousTokenClient::new(env, &mal_token_id);

    let admin = Address::generate(env);
    let name = String::from_str(env, "Test Group");

    kolo_client.initialize(
        &admin,
        &mal_token_id,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    (kolo_id, kolo_client, mal_token_id, mal_client)
}

/// Requirement: verify the CEI pattern is implemented — state (has_received_payout,
/// via NextPayoutIndex) must already reflect the payout by the time the external
/// token transfer callback executes.
#[test]
fn test_cei_effects_committed_before_external_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let (kolo_id, kolo_client, _mal_token_id, mal_client) = setup_with_malicious_token(&env);

    let member = Address::generate(&env);
    kolo_client.add_member(&member);

    mal_client.init(&kolo_id);
    mal_client.set_mode(&0u32);

    kolo_client.contribute(&member, &1000);
    assert!(!mal_client.get_observed()); // contribute's own transfer targets the contract, not a member

    kolo_client.payout(&member, &None);

    // The MaliciousToken observed, from *inside* the transfer() callback, that
    // has_received_payout(member) was already true — proving effects were
    // committed before the interaction, per CEI.
    assert!(mal_client.get_observed());
}

/// Requirement: a malicious token attempting to re-enter payout() during its
/// transfer() callback must be blocked by the reentrancy guard.
#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_reentrant_payout_call_is_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let (kolo_id, kolo_client, _mal_token_id, mal_client) = setup_with_malicious_token(&env);

    let member = Address::generate(&env);
    kolo_client.add_member(&member);

    mal_client.init(&kolo_id);

    // Contribute first while still in observe mode (mode 0) so the deposit
    // succeeds normally.
    kolo_client.contribute(&member, &1000);

    // Now arm the attack: the next transfer() call (triggered by payout())
    // will try to call payout() again for the same recipient.
    mal_client.set_mode(&1u32);

    // This should panic with "Reentrancy detected" because the guard set at
    // the top of payout() is still held when the reentrant call happens.
    kolo_client.payout(&member, &None);
}

/// Requirement: contribute() must also refuse to run if a payout is (somehow)
/// already mid-execution, as a defense-in-depth measure.
#[test]
#[should_panic(expected = "Reentrancy detected")]
fn test_contribute_blocked_while_payout_executing() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    // Simulate being mid-payout by setting the guard flag directly.
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::IsExecutingPayout, &true);
    });

    client.contribute(&member, &1000);
}

#[test]
#[should_panic(expected = "Contract is paused for emergency")]
fn test_pause_blocks_contribute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.pause();
    client.contribute(&member, &1000);
}

#[test]
#[should_panic(expected = "Error(Context, InvalidAction)")]
fn test_reentrant_payout_call_via_balance_is_blocked() {
    let env = Env::default();
    env.mock_all_auths();
    let (kolo_id, kolo_client, _mal_token_id, mal_client) = setup_with_malicious_token(&env);

    let member = Address::generate(&env);
    kolo_client.add_member(&member);
    mal_client.init(&kolo_id);

    kolo_client.contribute(&member, &1000);

    mal_client.set_mode(&2u32); // reenter via balance(), not transfer()
    kolo_client.payout(&member, &None);
}

#[test]
fn test_unpause_allows_contribute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.pause();
    client.unpause();
    client.contribute(&member, &1000);

    assert_eq!(client.get_contribution(&member), 1000);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_non_admin_cannot_pause() {
    let env = Env::default();

    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    // No auth mocked for pause — should fail since caller isn't the admin.
    let attacker = Address::generate(&env);
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &attacker,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "pause",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.pause();
}

#[test]
fn test_emergency_withdraw_full_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);

    client.contribute(&member1, &1000); // contract: 1000, CycleMemberCount frozen at 2
    client.contribute(&member2, &1000); // contract: 2000

    // Bug discovered / admin vanishes — pause and let member1 exit.
    client.pause();
    client.emergency_withdraw(&member1); // contract: 1000 (refunded to member1)

    assert_eq!(token_client.balance(&member1), 5000); // got their 1000 back
    assert_eq!(client.get_contribution(&member1), 0);

    env.as_contract(&contract_id, || {
        let count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count, 1); // decremented from 2 to 1
    });

    // Unpause: member1 re-contributes, and it's still their turn to be paid
    // (NextPayoutIndex never moved — emergency withdraw doesn't jump the queue).
    client.unpause();
    client.contribute(&member1, &1000); // contract: 2000 again

    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout(&member1, &None);

    // pool_size = contribution_amount(1000) * frozen CycleMemberCount(1) = 1000
    // member1: 4000 (after re-contribute) + 1000 (payout) = 5000
    assert_eq!(token_client.balance(&member1), 5000);
    assert!(client.has_received_payout(&member1));

    // member2's 1000 is still safely in the pool, untouched, waiting for their turn.
    assert_eq!(client.get_contribution(&member2), 1000);
    assert_eq!(client.get_next_payout_recipient(), member2);
}

#[test]
#[should_panic(expected = "No contribution to withdraw this cycle")]
fn test_emergency_withdraw_twice_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.pause();
    client.emergency_withdraw(&member);
    // Second call should panic — no contribution left this cycle.
    client.emergency_withdraw(&member);
}
