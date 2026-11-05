#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, IntoVal, String, Vec,
};

mod test;

const LEDGERS_TO_LIVE: u32 = 518_400; // ~30 days at 5s/ledger

fn extend_instance_ttl(env: &Env) {
    let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
    let cycle_len_ledgers = env
        .storage()
        .instance()
        .get::<_, u32>(&DataKey::CycleLengthLedgers)
        .unwrap_or(518_400);
    #[allow(clippy::unnecessary_cast)]
    let member_count: u32 = members.len() as u32;
    let rotation_ttl = cycle_len_ledgers.saturating_mul(member_count);
    let ttl = rotation_ttl.max(518_400);
    env.storage().instance().extend_ttl(ttl / 2, ttl);
}

fn extend_member_ttl(env: &Env, member: &Address) {
    let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
    let cycle_len_ledgers = env
        .storage()
        .instance()
        .get::<_, u32>(&DataKey::CycleLengthLedgers)
        .unwrap_or(518_400);
    #[allow(clippy::unnecessary_cast)]
    let member_count: u32 = members.len() as u32;
    let rotation_ttl = cycle_len_ledgers.saturating_mul(member_count);
    let ttl = rotation_ttl.max(518_400);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::Member(member.clone()), ttl / 2, ttl);
}

/// Reentrancy guard: panics if a payout is currently mid-execution.
/// Called at the top of any state-mutating entrypoint that could be reached
/// via a reentrant callback triggered from payout()'s token transfer.
fn assert_not_executing_payout(env: &Env) {
    let is_executing: bool = env
        .storage()
        .instance()
        .get(&DataKey::IsExecutingPayout)
        .unwrap_or(false);
    if is_executing {
        panic!("Reentrancy detected");
    }
}

fn require_not_paused(env: &Env) {
    let is_paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::IsPaused)
        .unwrap_or(false);
    if is_paused {
        panic!("Contract is paused for emergency");
    }
}

#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub enum GroupType {
    Rotational,
    GoalBased,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberState {
    pub total_contributions: i128,
    pub last_contribution_cycle_id: u32,
    pub has_received_payout: bool,
    pub current_cycle_contribution: i128,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Name,
    ContributionAmount,
    Members,
    Member(Address),
    NextPayoutIndex,
    CycleMemberCount,
    GroupType,
    TargetAmount,
    LockUntilTarget,
    CurrentCycleId,
    CycleLengthLedgers,
    /// Reentrancy guard mutex, set for the duration of payout()'s execution.
    IsExecutingPayout,
    IsPaused,
}

#[contracttype]
#[derive(Clone)]
pub struct User {
    pub wallet_address: Address,
    pub joined_groups: Vec<u32>,
}

#[contract]
pub struct KoloSavingsContract;

#[contractimpl]
impl KoloSavingsContract {
    /// Initialize the savings group
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        name: String,
        contribution_amount: i128,
        group_type: GroupType,
        target_amount: Option<i128>,
        lock_until_target: bool,
        expected_cycle_days: Option<u32>,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        if contribution_amount <= 0 {
            panic!("Contribution amount must be positive");
        }

        if contribution_amount > 1_000_000_000_000_000 {
            panic!("Contribution amount exceeds maximum limit");
        }

        admin.require_auth();

        let cycle_len = expected_cycle_days.unwrap_or(30) * 17_280;
        env.storage()
            .instance()
            .set(&DataKey::CycleLengthLedgers, &cycle_len);

        let empty_members: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::Members, &empty_members);

        extend_instance_ttl(&env);

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage()
            .instance()
            .set(&DataKey::ContributionAmount, &contribution_amount);
        env.storage()
            .instance()
            .set(&DataKey::GroupType, &group_type);
        if let Some(target) = target_amount {
            env.storage()
                .instance()
                .set(&DataKey::TargetAmount, &target);
        }
        env.storage()
            .instance()
            .set(&DataKey::LockUntilTarget, &lock_until_target);
        env.storage()
            .instance()
            .set(&DataKey::CurrentCycleId, &1u32);
        env.storage()
            .instance()
            .set(&DataKey::IsExecutingPayout, &false);

        env.events().publish(
            (symbol_short!("init"),),
            (admin, token, name, contribution_amount),
        );
    }

    /// Add a member to the group (Admin only)
    pub fn add_member(env: Env, new_member: Address) {
        require_not_paused(&env);
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth_for_args((new_member.clone(),).into_val(&env));
        extend_instance_ttl(&env);

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&new_member) {
            members.push_back(new_member.clone());
            env.storage().instance().set(&DataKey::Members, &members);

            // Initialize MemberState for the new member under a single key
            let state = MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Member(new_member.clone()), &state);

            // Initialize NextPayoutIndex if not already set
            if !env.storage().instance().has(&DataKey::NextPayoutIndex) {
                env.storage()
                    .instance()
                    .set(&DataKey::NextPayoutIndex, &0u32);
            }

            env.events()
                .publish((symbol_short!("add_mem"), new_member), ());
        }
    }

    /// Remove a member from the group (Admin only)
    /// Refunds current cycle contribution if applicable. Panics if member already received payout.
    pub fn remove_member(env: Env, member_to_remove: Address) {
        assert_not_executing_payout(&env);

        require_not_paused(&env);
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth_for_args((member_to_remove.clone(),).into_val(&env));
        extend_instance_ttl(&env);

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member_to_remove) {
            panic!("Not a member");
        }

        let members_list: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let remove_index = members_list
            .iter()
            .position(|m| m == member_to_remove)
            .unwrap() as u32;
        let next_payout_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        if remove_index < next_payout_index {
            panic!("Cannot remove member after their payout turn");
        }

        // Check if member contributed this cycle via MemberState cycle ID
        let member_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(member_to_remove.clone()))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });
        let current_cycle_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CurrentCycleId)
            .unwrap_or(1);
        let has_contributed_this_cycle =
            member_state.last_contribution_cycle_id == current_cycle_id;

        if has_contributed_this_cycle {
            let contribution_amount: i128 = env
                .storage()
                .instance()
                .get(&DataKey::ContributionAmount)
                .unwrap();
            let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(
                &env.current_contract_address(),
                &member_to_remove,
                &contribution_amount,
            );
        }

        // Remove the MemberState entry entirely
        env.storage()
            .persistent()
            .remove(&DataKey::Member(member_to_remove.clone()));

        // Adjust CycleMemberCount if present
        if env.storage().instance().has(&DataKey::CycleMemberCount) {
            let current_count: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CycleMemberCount)
                .unwrap();
            if current_count <= 1 {
                env.storage().instance().remove(&DataKey::CycleMemberCount);
            } else {
                env.storage()
                    .instance()
                    .set(&DataKey::CycleMemberCount, &(current_count - 1));
            }
        }

        let index = members.iter().position(|m| m == member_to_remove).unwrap() as u32;
        members.remove(index);
        env.storage().instance().set(&DataKey::Members, &members);

        env.events()
            .publish((symbol_short!("rm_member"), member_to_remove), ());
    }

    /// Contribute to the pool
    pub fn contribute(env: Env, member: Address, amount: i128) {
        assert_not_executing_payout(&env);

        require_not_paused(&env);
        member.require_auth();
        extend_instance_ttl(&env);

        let expected_amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::ContributionAmount)
            .unwrap();
        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);

        if group_type == GroupType::Rotational && amount != expected_amount {
            panic!("Must contribute the exact amount");
        }

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member) {
            panic!("Not a member");
        }

        // Freeze the member count at the start of a cycle on the first contribution
        if group_type == GroupType::Rotational
            && !env.storage().instance().has(&DataKey::CycleMemberCount)
        {
            let count = members.len() as i128;
            env.storage()
                .instance()
                .set(&DataKey::CycleMemberCount, &count);
        }

        let current_cycle_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CurrentCycleId)
            .unwrap_or(1);

        // Retrieve and check MemberState via cycle ID
        let mut member_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(member.clone()))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });

        if member_state.last_contribution_cycle_id == current_cycle_id {
            panic!("Already contributed this cycle");
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);

        // --- Effects (state updated before the external interaction below) ---
        member_state.total_contributions = member_state
            .total_contributions
            .checked_add(amount)
            .expect("Math overflow in contribution sum");
        member_state.last_contribution_cycle_id = current_cycle_id;
        member_state.current_cycle_contribution = amount;
        env.storage()
            .persistent()
            .set(&DataKey::Member(member.clone()), &member_state);

        extend_member_ttl(&env, &member);

        // --- Interaction ---
        // Transfer tokens from the member to this contract
        token_client.transfer(&member, env.current_contract_address(), &amount);

        env.events()
            .publish((symbol_short!("contrib"), member), amount);
    }

    /// Withdraw payout (Admin triggers payout to the next member in queue)
    /// Enforces strictly deterministic rotational payout (Ajo/Esusu) order.
    ///
    /// `amount` lets the admin explicitly request a *partial* payout — e.g.
    /// when not every member contributed this cycle and the group chooses to
    /// proceed with a reduced pool rather than wait. `None` pays the full
    /// computed pool (contribution_amount × the cycle's frozen member count),
    /// which is also the ceiling for an explicit `Some(amount)`: the admin can
    /// never authorize paying out more than the cycle's own pool, only less.
    /// Either way the amount is part of what the admin signs for, via
    /// `require_auth_for_args`, so a partial payout can't be substituted for
    /// a full one (or vice versa) after the fact.
    ///
    /// Follows the Checks-Effects-Interactions pattern: all validation happens
    /// first, then all state (NextPayoutIndex, the recipient's has_received_payout
    /// flag, and TTL extensions) is committed, and only then is the external
    /// token transfer performed. A reentrancy guard (`IsExecutingPayout`) is
    /// held for the duration of the call so that a malicious or upgraded token
    /// implementation cannot re-enter `payout()` or `contribute()` mid-transfer.
    pub fn payout(env: Env, expected_recipient: Address, amount: Option<i128>) {
        // --- Checks ---
        assert_not_executing_payout(&env);

        require_not_paused(&env);
        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);
        if group_type == GroupType::GoalBased {
            panic!("Payouts not allowed in GoalBased groups");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth_for_args((expected_recipient.clone(), amount).into_val(&env));
        extend_instance_ttl(&env);

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);

        if next_index >= members.len() {
            panic!("All members have received payouts this cycle");
        }

        let recipient: Address = members.get(next_index).unwrap();
        if recipient != expected_recipient {
            panic!("Recipient mismatch");
        }

        let contribution_amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::ContributionAmount)
            .unwrap();
        let frozen_count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .expect("No active cycle");
        let full_pool_size = contribution_amount
            .checked_mul(frozen_count)
            .expect("Math overflow in pool calculation");

        let pool_size = match amount {
            Some(requested) => {
                if requested <= 0 {
                    panic!("Payout amount must be positive");
                }
                if requested > full_pool_size {
                    panic!("Payout amount exceeds the full computed pool");
                }
                requested
            }
            None => full_pool_size,
        };

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);

        // --- Effects (all committed before the external call) ---
        env.storage()
            .instance()
            .set(&DataKey::IsExecutingPayout, &true);

        let contract_balance = token_client.balance(&env.current_contract_address());
        if pool_size > contract_balance {
            panic!("Insufficient funds in contract for requested payout");
        }

        env.storage()
            .instance()
            .set(&DataKey::NextPayoutIndex, &(next_index + 1));

        let mut recipient_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(recipient.clone()))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });
        recipient_state.has_received_payout = true;
        env.storage()
            .persistent()
            .set(&DataKey::Member(recipient.clone()), &recipient_state);

        extend_member_ttl(&env, &recipient);

        // --- Interaction ---
        token_client.transfer(&env.current_contract_address(), &recipient, &pool_size);

        // Release the guard now that the external call has returned.
        // (If the transfer above panics, the whole invocation is reverted by
        // the host and this line is never reached — the guard is never left
        // "stuck" true, since Soroban transactions are atomic.)
        env.storage()
            .instance()
            .set(&DataKey::IsExecutingPayout, &false);

        env.events()
            .publish((symbol_short!("payout"), recipient), pool_size);
    }

    /// Returns the address of the next member in line for a payout.
    pub fn get_next_payout_recipient(env: Env) -> Address {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        members
            .get(next_index)
            .expect("No members or cycle complete")
    }

    /// Withdraw savings (GoalBased groups only)
    pub fn withdraw_savings(env: Env, member: Address, amount: i128) {
        assert_not_executing_payout(&env);

        require_not_paused(&env);
        member.require_auth();
        extend_instance_ttl(&env);

        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);

        if group_type == GroupType::Rotational {
            panic!("Withdrawals not allowed in rotational groups");
        }

        if amount <= 0 {
            panic!("Withdrawal amount must be positive");
        }

        // Retrieve MemberState to check contributions
        let mut member_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(member.clone()))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });

        let current_contribution: i128 = member_state.total_contributions;

        if current_contribution < amount {
            panic!("Insufficient savings to withdraw");
        }

        let lock_until_target: bool = env
            .storage()
            .instance()
            .get(&DataKey::LockUntilTarget)
            .unwrap_or(false);

        if lock_until_target {
            if let Some(target_amount) = env
                .storage()
                .instance()
                .get::<_, i128>(&DataKey::TargetAmount)
            {
                if current_contribution < target_amount {
                    panic!("Target amount not reached yet");
                }
            }
        }

        let new_contribution = current_contribution
            .checked_sub(amount)
            .expect("Math underflow in withdrawal");
        member_state.total_contributions = new_contribution;
        env.storage()
            .persistent()
            .set(&DataKey::Member(member.clone()), &member_state);
        env.storage().persistent().extend_ttl(
            &DataKey::Member(member.clone()),
            LEDGERS_TO_LIVE / 2,
            LEDGERS_TO_LIVE,
        );

        // --- Interaction ---
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &member, &amount);

        env.events()
            .publish((symbol_short!("withdraw"), member), amount);
    }

    /// Emergency withdraw (only usable while paused). Lets a member reclaim exactly
    /// what they contributed in the *current* cycle, and unwinds the cycle counters
    /// so the pool math stays correct if the contract is later unpaused.
    pub fn emergency_withdraw(env: Env, member: Address) {
        member.require_auth();

        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false);
        if !is_paused {
            panic!("Contract is not paused");
        }

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member) {
            panic!("Not a member");
        }

        let current_cycle_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CurrentCycleId)
            .unwrap_or(1);

        let mut member_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(member.clone()))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });

        if member_state.last_contribution_cycle_id != current_cycle_id
            || member_state.current_cycle_contribution <= 0
        {
            panic!("No contribution to withdraw this cycle");
        }

        let amount = member_state.current_cycle_contribution;

        // Unwind this member's contribution — can never withdraw more than they put in,
        // because we only ever refund exactly current_cycle_contribution.
        member_state.total_contributions = member_state
            .total_contributions
            .checked_sub(amount)
            .expect("Integer underflow in contribution total");
        member_state.current_cycle_contribution = 0;
        member_state.last_contribution_cycle_id = 0; // clears "has contributed this cycle"
        env.storage()
            .persistent()
            .set(&DataKey::Member(member.clone()), &member_state);

        // Keep the frozen rotational pool size consistent: one fewer paid-in member.
        if env.storage().instance().has(&DataKey::CycleMemberCount) {
            let current_count: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CycleMemberCount)
                .unwrap();
            if current_count <= 1 {
                env.storage().instance().remove(&DataKey::CycleMemberCount);
            } else {
                env.storage()
                    .instance()
                    .set(&DataKey::CycleMemberCount, &(current_count - 1));
            }
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &member, &amount);

        extend_member_ttl(&env, &member);

        env.events()
            .publish((symbol_short!("emg_wd"), member), amount);
    }

    /// Pause the contract (Admin only). Blocks contribute, payout, add_member, reset_cycle.
    pub fn pause(env: Env) {
        require_not_paused(&env);
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);
        env.storage().instance().set(&DataKey::IsPaused, &true);
        env.events().publish((symbol_short!("pause"),), ());
    }

    /// Unpause the contract (Admin only).
    pub fn unpause(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);
        env.storage().instance().set(&DataKey::IsPaused, &false);
        env.events().publish((symbol_short!("unpause"),), ());
    }

    /// Resets the payout cycle so members can contribute and receive payouts again.
    /// NextPayoutIndex persists across the full rotation — it only resets when
    /// all members have received their payout and the admin triggers a new rotation.
    pub fn reset_cycle(env: Env) {
        assert_not_executing_payout(&env);

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth_for_args(().into_val(&env));
        extend_instance_ttl(&env);

        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);

        // Increment cycle ID — this automatically makes last_contribution_cycle_id
        // comparisons evaluate to "not contributed this cycle" for the next cycle.
        let current_cycle_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CurrentCycleId)
            .unwrap_or(1);
        env.storage()
            .instance()
            .set(&DataKey::CurrentCycleId, &(current_cycle_id + 1));

        if group_type == GroupType::Rotational {
            env.storage().instance().remove(&DataKey::CycleMemberCount);
        }

        env.events().publish((symbol_short!("reset"),), ());
    }

    /// Resets the payout queue so the rotation starts from the first member again.
    /// Call this after all members have received their payout to begin a new rotation.
    pub fn reset_rotation(env: Env) {
        assert_not_executing_payout(&env);

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth_for_args(().into_val(&env));
        extend_instance_ttl(&env);

        env.storage()
            .instance()
            .set(&DataKey::NextPayoutIndex, &0u32);

        env.events().publish((symbol_short!("new_rot"),), ());
    }

    /// Get contract balance
    pub fn get_balance(env: Env) -> i128 {
        extend_instance_ttl(&env);
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.balance(&env.current_contract_address())
    }

    pub fn get_contribution(env: Env, member: Address) -> i128 {
        env.storage().persistent().extend_ttl(
            &DataKey::Member(member.clone()),
            LEDGERS_TO_LIVE / 2,
            LEDGERS_TO_LIVE,
        );
        let member_state: MemberState = env
            .storage()
            .persistent()
            .get(&DataKey::Member(member))
            .unwrap_or(MemberState {
                total_contributions: 0,
                last_contribution_cycle_id: 0,
                has_received_payout: false,
                current_cycle_contribution: 0,
            });
        member_state.total_contributions
    }

    pub fn has_received_payout(env: Env, member: Address) -> bool {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_payout_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        match members.iter().position(|m| m == member) {
            Some(idx) => (idx as u32) < next_payout_index,
            None => false,
        }
    }
}
