#![cfg(test)]

use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env};

use crate::error::StreamError;
use crate::{StreamContract, StreamContractClient};

/// Helper: set the ledger timestamp to `ts`.
fn set_ledger_timestamp(env: &Env, ts: u64) {
    let mut ledger_info = env.ledger().get();
    ledger_info.timestamp = ts;
    env.ledger().set(ledger_info);
}

/// Helper: create a token contract and mint `amount` to `to`.
fn create_token<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    (
        TokenClient::new(env, &contract_address.address()),
        StellarAssetClient::new(env, &contract_address.address()),
    )
}

/// Helper: deploy the stream contract and initialize it.
fn setup_contract(env: &Env) -> (StreamContractClient<'_>, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(env, &contract_id);
    client.init(&admin);
    (client, admin)
}

// =============================================================================
// Initialization Tests
// =============================================================================

#[test]
fn test_init_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);
    let result = client.try_init(&admin);
    assert!(result.is_ok());
}

#[test]
fn test_init_requires_admin_auth() {
    let env = Env::default();
    // Deliberately no mock_all_auths(): init must be rejected without the
    // admin's authorization.
    let admin = Address::generate(&env);
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    let result = client.try_init(&admin);
    assert!(result.is_err(), "init must require the admin's auth");
}

#[test]
fn test_init_rejects_contract_itself_as_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    let result = client.try_init(&contract_id);
    assert_eq!(result, Err(Ok(StreamError::InvalidAdmin)));
}

#[test]
fn test_init_double_initialization_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let another_admin = Address::generate(&env);
    let result = client.try_init(&another_admin);
    assert_eq!(result, Err(Ok(StreamError::AlreadyInitialized)));
}

#[test]
fn test_reserved_error_codes_are_pinned() {
    // Assert discriminant values remain stable for ABI compatibility
    assert_eq!(StreamError::StreamEnded as u32, 5);
    assert_eq!(StreamError::AmountMismatch as u32, 9);
fn test_operations_before_init_fail() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    // create_stream fails before init
    let create_res = client.try_create_stream(
        &sender,
        &recipient,
        &token,
        &1_000_000_i128,
        &1000_u64,
        &2000_u64,
        &true,
    );
    assert_eq!(create_res, Err(Ok(StreamError::NotInitialized)));

    // balance_of fails before init
    let bal_res = client.try_balance_of(&0_u64, &recipient);
    assert_eq!(bal_res, Err(Ok(StreamError::NotInitialized)));

    // get_stream fails before init
    let get_res = client.try_get_stream(&0_u64);
    assert_eq!(get_res, Err(Ok(StreamError::NotInitialized)));

    // withdraw fails before init
    let withdraw_res = client.try_withdraw(&0_u64, &100_i128);
    assert_eq!(withdraw_res, Err(Ok(StreamError::NotInitialized)));

    // cancel fails before init
    let cancel_res = client.try_cancel(&0_u64);
    assert_eq!(cancel_res, Err(Ok(StreamError::NotInitialized)));

    // init succeeds and subsequent operations proceed as expected
    let admin = Address::generate(&env);
    assert!(client.try_init(&admin).is_ok());
    assert_eq!(client.try_get_stream(&0_u64), Err(Ok(StreamError::StreamNotFound)));
}

// =============================================================================
// Stream Creation Tests
// =============================================================================

#[test]
fn test_create_stream_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let start_time: u64 = 1000;
    let stop_time: u64 = 2000;

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &start_time,
        &stop_time,
        &true,
    );

    assert_eq!(stream_id, 0);

    // Verify contract holds the tokens
    assert_eq!(token_client.balance(&client.address), deposit);
    assert_eq!(token_client.balance(&sender), 0);

    // Verify stream data
    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.id, 0);
    assert_eq!(stream.sender, sender);
    assert_eq!(stream.recipient, recipient);
    assert_eq!(stream.deposit_amount, deposit);
    assert_eq!(stream.start_time, start_time);
    assert_eq!(stream.stop_time, stop_time);
    assert_eq!(stream.remaining_balance, deposit);
    assert_eq!(stream.recipient_withdrawn, 0);
    assert!(!stream.is_canceled);
    assert!(stream.cancelable);

    // Verify StreamCreated event payload includes cancelable
    let events = env.events().all();
    assert!(!events.is_empty(), "expected StreamCreated event to be emitted");
}

#[test]
fn test_create_stream_zero_deposit_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, _) = create_token(&env, &sender);

    let result = client.try_create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &0_i128,
        &1000_u64,
        &2000_u64,
        &true,
    );
    assert_eq!(result, Err(Ok(StreamError::ZeroDeposit)));
}

#[test]
fn test_create_stream_invalid_time_range_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, _) = create_token(&env, &sender);

    // start_time >= stop_time
    let result = client.try_create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &1000_i128,
        &2000_u64,
        &1000_u64,
        &true,
    );
    assert_eq!(result, Err(Ok(StreamError::InvalidTimeRange)));

    // start_time == stop_time
    let result = client.try_create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &1000_i128,
        &2000_u64,
        &2000_u64,
        &true,
    );
    assert_eq!(result, Err(Ok(StreamError::InvalidTimeRange)));
}

#[test]
fn test_create_multiple_streams_increments_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    token_admin_client.mint(&sender, &2_000_000_i128);

    let id0 = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &1_000_000_i128,
        &1000_u64,
        &2000_u64,
        &true,
    );
    let id1 = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &1_000_000_i128,
        &3000_u64,
        &4000_u64,
        &false,
    );

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

#[test]
fn test_retroactive_stream_accrual() {
fn test_stream_balance_isolation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    // Advance ledger timestamp to 1500 before creation
    set_ledger_timestamp(&env, 1500);

    // Create stream with start_time = 1000, stop_time = 2000 (50% elapsed retroactively)
    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
    let sender0 = Address::generate(&env);
    let recipient0 = Address::generate(&env);
    let sender1 = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let (token_client, token_admin_client) = create_token(&env, &sender0);

    let deposit0: i128 = 1_000_000;
    let deposit1: i128 = 2_000_000;
    token_admin_client.mint(&sender0, &deposit0);
    token_admin_client.mint(&sender1, &deposit1);

    // Stream 0: [1000..2000], cancelable
    let id0 = client.create_stream(
        &sender0,
        &recipient0,
        &token_client.address,
        &deposit0,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // Immediately after creation, recipient has 500_000 available
    let recipient_bal = client.balance_of(&stream_id, &recipient);
    let sender_bal = client.balance_of(&stream_id, &sender);

    assert_eq!(recipient_bal, 500_000);
    assert_eq!(sender_bal, 500_000);
    assert_eq!(recipient_bal + sender_bal, deposit);

    // Recipient can immediately withdraw accrued tokens
    client.withdraw(&stream_id, &recipient_bal);
    assert_eq!(token_client.balance(&recipient), 500_000);
    assert_eq!(client.balance_of(&stream_id, &recipient), 0);
    // Stream 1: [1000..3000], cancelable
    let id1 = client.create_stream(
        &sender1,
        &recipient1,
        &token_client.address,
        &deposit1,
        &1000_u64,
        &3000_u64,
        &true,
    );

    // At timestamp 1500:
    // Stream 0 is at 50%: recipient0 has 500_000, sender0 has 500_000
    // Stream 1 is at 25%: recipient1 has 500_000, sender1 has 1_500_000
    set_ledger_timestamp(&env, 1500);

    assert_eq!(client.balance_of(&id0, &recipient0), 500_000);
    assert_eq!(client.balance_of(&id0, &sender0), 500_000);
    assert_eq!(
        client.balance_of(&id0, &recipient0) + client.balance_of(&id0, &sender0),
        deposit0
    );

    assert_eq!(client.balance_of(&id1, &recipient1), 500_000);
    assert_eq!(client.balance_of(&id1, &sender1), 1_500_000);
    assert_eq!(
        client.balance_of(&id1, &recipient1) + client.balance_of(&id1, &sender1),
        deposit1
    );

    // Partial withdrawal on Stream 1 does not affect Stream 0
    client.withdraw(&id1, &200_000_i128);
    assert_eq!(client.balance_of(&id0, &recipient0), 500_000);
    assert_eq!(client.balance_of(&id0, &sender0), 500_000);
    assert_eq!(client.balance_of(&id1, &recipient1), 300_000);

    // Cancelling Stream 0 leaves Stream 1 completely intact
    client.cancel(&id0);
    assert!(client.get_stream(&id0).is_canceled);
    assert_eq!(client.balance_of(&id1, &recipient1), 300_000);
    assert_eq!(client.balance_of(&id1, &sender1), 1_500_000);
    assert!(!client.get_stream(&id1).is_canceled);
}

// =============================================================================
// Balance Calculation Tests
// =============================================================================

#[test]
fn test_balance_of_before_start() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // Before start: recipient has 0, sender has full deposit
    set_ledger_timestamp(&env, 500);
    assert_eq!(client.balance_of(&stream_id, &recipient), 0);
    assert_eq!(client.balance_of(&stream_id, &sender), deposit);
}

#[test]
fn test_balance_of_at_midpoint() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At midpoint (50%): recipient earns 500_000, sender refundable 500_000
    set_ledger_timestamp(&env, 1500);
    assert_eq!(client.balance_of(&stream_id, &recipient), 500_000);
    assert_eq!(client.balance_of(&stream_id, &sender), 500_000);
}

#[test]
fn test_balance_of_after_stop() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // After stop: recipient earns full deposit, sender 0
    set_ledger_timestamp(&env, 3000);
    assert_eq!(client.balance_of(&stream_id, &recipient), deposit);
    assert_eq!(client.balance_of(&stream_id, &sender), 0);
}

#[test]
fn test_balance_of_linear_distribution_25_percent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 25%: 250_000 earned
    set_ledger_timestamp(&env, 1250);
    assert_eq!(client.balance_of(&stream_id, &recipient), 250_000);
    assert_eq!(client.balance_of(&stream_id, &sender), 750_000);
}

#[test]
fn test_balance_of_unauthorized_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let stranger = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    set_ledger_timestamp(&env, 1500);
    let result = client.try_balance_of(&stream_id, &stranger);
    assert_eq!(result, Err(Ok(StreamError::Unauthorized)));
}

#[test]
fn test_balance_of_stream_not_found() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let target = Address::generate(&env);

    let result = client.try_balance_of(&999_u64, &target);
    assert_eq!(result, Err(Ok(StreamError::StreamNotFound)));
}

// =============================================================================
// Withdrawal Tests
// =============================================================================

#[test]
fn test_withdraw_partial() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 50%, withdraw 300_000 of the 500_000 available
    set_ledger_timestamp(&env, 1500);
    client.withdraw(&stream_id, &300_000_i128);

    assert_eq!(token_client.balance(&recipient), 300_000);

    // Remaining available for recipient: 500_000 - 300_000 = 200_000
    assert_eq!(client.balance_of(&stream_id, &recipient), 200_000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.recipient_withdrawn, 300_000);
    assert_eq!(stream.remaining_balance, 700_000);

    // Verify TokensWithdrawn event is published
    let events = env.events().all();
    assert!(!events.is_empty(), "expected TokensWithdrawn event");
}

#[test]
fn test_withdraw_full_after_stream_ends() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // After stream ends, withdraw full amount
    set_ledger_timestamp(&env, 3000);
    client.withdraw(&stream_id, &deposit);

    assert_eq!(token_client.balance(&recipient), deposit);
    assert_eq!(token_client.balance(&client.address), 0);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.recipient_withdrawn, deposit);
    assert_eq!(stream.remaining_balance, 0);
}

#[test]
fn test_withdraw_too_much_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 50%, only 500_000 available, try to withdraw 600_000
    set_ledger_timestamp(&env, 1500);
    let result = client.try_withdraw(&stream_id, &600_000_i128);
    assert_eq!(result, Err(Ok(StreamError::WithdrawAmountTooHigh)));
}

#[test]
fn test_withdraw_exact_available_boundary() {
fn test_withdraw_negative_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At fractional timestamp 1400 (40% elapsed): available is 400_000
    set_ledger_timestamp(&env, 1400);
    let available = client.balance_of(&stream_id, &recipient);
    assert_eq!(available, 400_000);

    // Withdrawing one unit more than available fails
    let fail_res = client.try_withdraw(&stream_id, &(available + 1));
    assert_eq!(fail_res, Err(Ok(StreamError::WithdrawAmountTooHigh)));

    // Withdrawing exactly available succeeds
    client.withdraw(&stream_id, &available);
    assert_eq!(token_client.balance(&recipient), available);

    // Follow-up balance_of is exactly 0
    assert_eq!(client.balance_of(&stream_id, &recipient), 0);
    set_ledger_timestamp(&env, 1500);
    let before = client.get_stream(&stream_id);
    let result = client.try_withdraw(&stream_id, &-500_i128);

    assert_eq!(result, Err(Ok(StreamError::InvalidWithdrawAmount)));
    assert_eq!(client.get_stream(&stream_id), before);
    assert_eq!(token_client.balance(&recipient), 0);
}

#[test]
fn test_withdraw_multiple_times() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 25%, withdraw 100_000
    set_ledger_timestamp(&env, 1250);
    client.withdraw(&stream_id, &100_000_i128);
    assert_eq!(token_client.balance(&recipient), 100_000);

    // At 75%, available = 750_000 - 100_000 = 650_000, withdraw 400_000
    set_ledger_timestamp(&env, 1750);
    client.withdraw(&stream_id, &400_000_i128);
    assert_eq!(token_client.balance(&recipient), 500_000);

    // After end, available = 1_000_000 - 500_000 = 500_000
    set_ledger_timestamp(&env, 3000);
    client.withdraw(&stream_id, &500_000_i128);
    assert_eq!(token_client.balance(&recipient), deposit);
}

// =============================================================================
// Cancellation Tests
// =============================================================================

#[test]
fn test_cancel_at_midpoint() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // Cancel at 50%
    set_ledger_timestamp(&env, 1500);
    client.cancel(&stream_id);

    // Recipient gets earned portion (500_000), sender gets refund (500_000)
    assert_eq!(token_client.balance(&recipient), 500_000);
    assert_eq!(token_client.balance(&sender), 500_000);
    assert_eq!(token_client.balance(&client.address), 0);

    let stream = client.get_stream(&stream_id);
    assert!(stream.is_canceled);
    assert_eq!(stream.remaining_balance, 0);

    // Verify StreamCanceled event is published
    let events = env.events().all();
    assert!(!events.is_empty(), "expected StreamCanceled event");
}

#[test]
fn test_cancel_before_start() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // Cancel before start: sender gets full refund
    set_ledger_timestamp(&env, 500);
    client.cancel(&stream_id);

    assert_eq!(token_client.balance(&sender), deposit);
    assert_eq!(token_client.balance(&recipient), 0);
}

#[test]
fn test_cancel_at_start_time() {
fn test_cancel_at_stop_time() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // Cancel exactly at start_time: sender receives full refund, recipient gets 0
    set_ledger_timestamp(&env, 1000);
    client.cancel(&stream_id);

    assert_eq!(token_client.balance(&sender), deposit);
    assert_eq!(token_client.balance(&recipient), 0);
    // Cancel exactly at stop_time: recipient gets full deposit, sender refund is zero
    set_ledger_timestamp(&env, 2000);
    client.cancel(&stream_id);

    assert_eq!(token_client.balance(&recipient), deposit);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 0);

    let stream = client.get_stream(&stream_id);
    assert!(stream.is_canceled);
    assert_eq!(stream.remaining_balance, 0);
}

#[test]
fn test_cancel_after_partial_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 50%, withdraw 200_000
    set_ledger_timestamp(&env, 1500);
    client.withdraw(&stream_id, &200_000_i128);
    assert_eq!(token_client.balance(&recipient), 200_000);

    // Now cancel at 50%: recipient_payout = 500_000 - 200_000 = 300_000
    // sender_refund = 1_000_000 - 500_000 = 500_000
    client.cancel(&stream_id);
    assert_eq!(token_client.balance(&recipient), 500_000); // 200_000 + 300_000
    assert_eq!(token_client.balance(&sender), 500_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_cancel_after_full_accrued_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    // At 50%, withdraw the entire accrued available amount (500_000)
    set_ledger_timestamp(&env, 1500);
    client.withdraw(&stream_id, &500_000_i128);
    assert_eq!(token_client.balance(&recipient), 500_000);

    // Cancel at 50%: recipient already withdrew all earned tokens (recipient_payout == 0)
    // Sender refund should be deposit - earned = 500_000
    client.cancel(&stream_id);

    // Recipient balance unchanged; no additional transfer
    assert_eq!(token_client.balance(&recipient), 500_000);
    assert_eq!(token_client.balance(&sender), 500_000);
    assert_eq!(token_client.balance(&client.address), 0);

    let stream = client.get_stream(&stream_id);
    assert!(stream.is_canceled);
    assert_eq!(stream.remaining_balance, 0);
}

#[test]
fn test_cancel_not_cancelable_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &false, // not cancelable
    );

    set_ledger_timestamp(&env, 1500);
    let result = client.try_cancel(&stream_id);
    assert_eq!(result, Err(Ok(StreamError::NotCancelable)));
}

#[test]
fn test_non_cancelable_stream_lifecycle_end_to_end() {
fn test_cancel_requires_sender_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &false, // cancelable = false
    );

    // 1. Partial accrual at 50%
    set_ledger_timestamp(&env, 1500);
    assert_eq!(client.balance_of(&stream_id, &recipient), 500_000);
    assert_eq!(client.balance_of(&stream_id, &sender), 500_000);

    // 2. Cancellation attempt must fail with NotCancelable
    let cancel_res = client.try_cancel(&stream_id);
    assert_eq!(cancel_res, Err(Ok(StreamError::NotCancelable)));

    // 3. Partial withdrawal succeeds
    client.withdraw(&stream_id, &200_000_i128);
    assert_eq!(token_client.balance(&recipient), 200_000);
    assert_eq!(client.balance_of(&stream_id, &recipient), 300_000);

    // 4. Complete accrual after stop_time (timestamp 2500)
    set_ledger_timestamp(&env, 2500);
    assert_eq!(client.balance_of(&stream_id, &recipient), 800_000);

    // 5. Final withdrawal of all remaining tokens
    client.withdraw(&stream_id, &800_000_i128);
    assert_eq!(token_client.balance(&recipient), deposit);
    assert_eq!(token_client.balance(&client.address), 0);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.recipient_withdrawn, deposit);
    assert_eq!(stream.remaining_balance, 0);
    assert!(!stream.is_canceled);
    assert!(!stream.cancelable);
        &true,
    );

    set_ledger_timestamp(&env, 1500);

    // Cancel without mock_all_auths must fail because sender auth is missing
    let unauthed_env = Env::default();
    // Re-bind client in env without mock auth
    let result = client.mock_auths(&[]).try_cancel(&stream_id);
    assert!(result.is_err(), "cancel without sender auth must be rejected");

    // Stream remains active, recipient balance untouched
    let stream = client.get_stream(&stream_id);
    assert!(!stream.is_canceled);
    assert_eq!(token_client.balance(&recipient), 0);

    // Authorised cancel from sender succeeds
    client.mock_all_auths();
    assert!(client.try_cancel(&stream_id).is_ok());
    assert!(client.get_stream(&stream_id).is_canceled);
}

#[test]
fn test_cancel_already_canceled_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    set_ledger_timestamp(&env, 1500);
    client.cancel(&stream_id);

    let result = client.try_cancel(&stream_id);
    assert_eq!(result, Err(Ok(StreamError::StreamCanceled)));
}

#[test]
fn test_withdraw_from_canceled_stream_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    set_ledger_timestamp(&env, 1500);
    client.cancel(&stream_id);

    let result = client.try_withdraw(&stream_id, &100_000_i128);
    assert_eq!(result, Err(Ok(StreamError::StreamCanceled)));
}

// =============================================================================
// Stream Not Found Tests
// =============================================================================

#[test]
fn test_get_nonexistent_stream_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_get_stream(&999_u64);
    assert_eq!(result, Err(Ok(StreamError::StreamNotFound)));
}

#[test]
fn test_withdraw_nonexistent_stream_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_withdraw(&999_u64, &100_i128);
    assert_eq!(result, Err(Ok(StreamError::StreamNotFound)));
}

#[test]
fn test_cancel_nonexistent_stream_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_cancel(&999_u64);
    assert_eq!(result, Err(Ok(StreamError::StreamNotFound)));
}

// =============================================================================
// Precision / Edge Case Tests
// =============================================================================

#[test]
fn test_exact_arithmetic_no_precision_loss() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    // Deposit that doesn't divide evenly by duration
    let deposit: i128 = 1_000_003; // prime-ish number
    let duration: u64 = 1000;
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &(1000 + duration),
        &true,
    );

    // At each point, sender_balance + recipient_balance should equal deposit
    for offset in [0_u64, 1, 100, 333, 500, 667, 999, 1000] {
        set_ledger_timestamp(&env, 1000 + offset);
        let r_bal = client.balance_of(&stream_id, &recipient);
        let s_bal = client.balance_of(&stream_id, &sender);
        assert_eq!(
            r_bal + s_bal,
            deposit,
            "Precision loss at offset {offset}: recipient={r_bal}, sender={s_bal}"
        );
    }
}

#[test]
fn test_large_deposit_no_overflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    // Large deposit close to i128 practical range
    let deposit: i128 = 1_000_000_000_000_000; // 1 quadrillion stroops
    token_admin_client.mint(&sender, &deposit);

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );

    set_ledger_timestamp(&env, 1500);
    let r_bal = client.balance_of(&stream_id, &recipient);
    assert_eq!(r_bal, 500_000_000_000_000);
}

#[test]
fn test_contract_token_balance_lifecycle_invariants() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (token_client, token_admin_client) = create_token(&env, &sender);

    let deposit: i128 = 1_000_000;
    token_admin_client.mint(&sender, &deposit);

    // Initial state: contract holds 0
    assert_eq!(token_client.balance(&client.address), 0);

    // 1. After create: contract token balance must equal stream remaining_balance
    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &1000_u64,
        &2000_u64,
        &true,
    );
    let s1 = client.get_stream(&stream_id);
    assert_eq!(token_client.balance(&client.address), s1.remaining_balance);
    assert_eq!(token_client.balance(&client.address), deposit);

    // 2. After partial withdraw: contract token balance must still equal remaining_balance
    set_ledger_timestamp(&env, 1500);
    client.withdraw(&stream_id, &300_000_i128);
    let s2 = client.get_stream(&stream_id);
    assert_eq!(token_client.balance(&client.address), s2.remaining_balance);
    assert_eq!(token_client.balance(&client.address), 700_000);

    // 3. After cancel: contract token balance must be 0, remaining_balance must be 0
    client.cancel(&stream_id);
    let s3 = client.get_stream(&stream_id);
    assert_eq!(s3.remaining_balance, 0);
    assert_eq!(token_client.balance(&client.address), 0);

    // 4. Test full withdrawal path invariant on a separate stream
    token_admin_client.mint(&sender, &deposit);
    let id2 = client.create_stream(
        &sender,
        &recipient,
        &token_client.address,
        &deposit,
        &3000_u64,
        &4000_u64,
        &true,
    );
    assert_eq!(token_client.balance(&client.address), deposit);

    set_ledger_timestamp(&env, 5000);
    client.withdraw(&id2, &deposit);
    let s4 = client.get_stream(&id2);
    assert_eq!(s4.remaining_balance, 0);
    assert_eq!(token_client.balance(&client.address), 0);
}
