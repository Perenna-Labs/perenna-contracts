use soroban_sdk::{contractevent, Address, Env};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamCreated {
    pub sender: Address,
    pub recipient: Address,
    pub stream_id: u64,
    pub token: Address,
    pub deposit_amount: i128,
    pub start_time: u64,
    pub stop_time: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensWithdrawn {
    pub recipient: Address,
    pub stream_id: u64,
    pub amount: i128,
    pub remaining_balance: i128,
    pub recipient_withdrawn: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamCanceled {
    pub stream_id: u64,
    pub sender_refund: i128,
    pub recipient_payout: i128,
}

#[allow(clippy::too_many_arguments)]
pub fn emit_stream_created(
    env: &Env,
    sender: &Address,
    recipient: &Address,
    stream_id: u64,
    token: &Address,
    deposit_amount: i128,
    start_time: u64,
    stop_time: u64,
) {
    env.events().publish_event(&StreamCreated {
        sender: sender.clone(),
        recipient: recipient.clone(),
        stream_id,
        token: token.clone(),
        deposit_amount,
        start_time,
        stop_time,
    });
}

pub fn emit_tokens_withdrawn(
    env: &Env,
    recipient: &Address,
    stream_id: u64,
    amount: i128,
    remaining_balance: i128,
    recipient_withdrawn: i128,
) {
    env.events().publish_event(&TokensWithdrawn {
        recipient: recipient.clone(),
        stream_id,
        amount,
        remaining_balance,
        recipient_withdrawn,
    });
}

pub fn emit_stream_canceled(
    env: &Env,
    stream_id: u64,
    sender_refund: i128,
    recipient_payout: i128,
) {
    env.events().publish_event(&StreamCanceled {
        stream_id,
        sender_refund,
        recipient_payout,
    });
}
