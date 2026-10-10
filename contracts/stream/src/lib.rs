#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env};

pub mod admin;
pub mod error;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use crate::error::StreamError;
use crate::types::Stream;

#[contract]
pub struct StreamContract;

/// Computes the total amount earned by the recipient at a given timestamp.
/// Uses exact integer arithmetic: earned = deposit_amount * elapsed / duration.
fn compute_earned(stream: &Stream, current_time: u64) -> Result<i128, StreamError> {
    if current_time <= stream.start_time {
        return Ok(0);
    }
    if current_time >= stream.stop_time {
        return Ok(stream.deposit_amount);
    }
    let elapsed = current_time
        .checked_sub(stream.start_time)
        .ok_or(StreamError::MathOverflow)?;
    let duration = stream
        .stop_time
        .checked_sub(stream.start_time)
        .ok_or(StreamError::MathOverflow)?;

    let elapsed_i128 = elapsed as i128;
    let duration_i128 = duration as i128;

    let earned = stream
        .deposit_amount
        .checked_mul(elapsed_i128)
        .ok_or(StreamError::MathOverflow)?
        .checked_div(duration_i128)
        .ok_or(StreamError::MathOverflow)?;

    Ok(earned)
}

fn require_initialized(env: &Env) -> Result<(), StreamError> {
    if storage::has_admin(env) {
        Ok(())
    } else {
        Err(StreamError::NotInitialized)
    }
}

#[contractimpl]
impl StreamContract {
    /// Initialize the contract with an admin address. Can only be called once.
    pub fn init(env: Env, admin: Address) -> Result<(), StreamError> {
        admin::initialize(&env, &admin)
    }

    /// Create a new payment stream. The sender's tokens are transferred into
    /// the contract immediately. Returns the new stream ID.
    #[allow(clippy::too_many_arguments)]
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token_addr: Address,
        deposit_amount: i128,
        start_time: u64,
        stop_time: u64,
        cancelable: bool,
    ) -> Result<u64, StreamError> {
        require_initialized(&env)?;

        sender.require_auth();

        if deposit_amount <= 0 {
            return Err(StreamError::ZeroDeposit);
        }
        if start_time >= stop_time {
            return Err(StreamError::InvalidTimeRange);
        }

        // Informational rate for clients. The remainder from integer division
        // does not affect distribution, which uses the exact earned formula
        // (deposit_amount * elapsed / duration).
        let duration_i128 = stop_time
            .checked_sub(start_time)
            .ok_or(StreamError::MathOverflow)? as i128;
        let rate_per_second = deposit_amount
            .checked_div(duration_i128)
            .ok_or(StreamError::MathOverflow)?;

        // Transfer tokens from sender to contract
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&sender, env.current_contract_address(), &deposit_amount);

        let stream_id = storage::get_next_stream_id(&env);
        let next_id = stream_id.checked_add(1).ok_or(StreamError::MathOverflow)?;
        storage::set_next_stream_id(&env, next_id);

        let stream = Stream {
            id: stream_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token_addr.clone(),
            deposit_amount,
            start_time,
            stop_time,
            rate_per_second,
            remaining_balance: deposit_amount,
            recipient_withdrawn: 0,
            is_canceled: false,
            cancelable,
        };

        storage::set_stream(&env, &stream);

        events::emit_stream_created(
            &env,
            &sender,
            &recipient,
            stream_id,
            &token_addr,
            deposit_amount,
            start_time,
            stop_time,
        );

        Ok(stream_id)
    }

    /// Compute the current available balance for a given address on a stream.
    /// For the recipient: earned - already_withdrawn
    /// For the sender: deposit_amount - earned (refundable portion)
    pub fn balance_of(env: Env, stream_id: u64, target: Address) -> Result<i128, StreamError> {
        require_initialized(&env)?;
        let stream = storage::get_stream(&env, stream_id)?;
        storage::extend_stream_ttl(&env, stream_id);

        if stream.is_canceled {
            return Err(StreamError::StreamCanceled);
        }

        let current_time = env.ledger().timestamp();
        let earned = compute_earned(&stream, current_time)?;

        if target == stream.recipient {
            let available = earned
                .checked_sub(stream.recipient_withdrawn)
                .ok_or(StreamError::MathOverflow)?;
            Ok(available)
        } else if target == stream.sender {
            let refundable = stream
                .deposit_amount
                .checked_sub(earned)
                .ok_or(StreamError::MathOverflow)?;
            Ok(refundable)
        } else {
            Err(StreamError::Unauthorized)
        }
    }

    /// Retrieve the full stream data struct by ID.
    pub fn get_stream(env: Env, stream_id: u64) -> Result<Stream, StreamError> {
        require_initialized(&env)?;
        let stream = storage::get_stream(&env, stream_id)?;
        storage::extend_stream_ttl(&env, stream_id);
        Ok(stream)
    }

    /// Withdraw accrued tokens from a stream. Only the recipient may call this.
    pub fn withdraw(env: Env, stream_id: u64, amount: i128) -> Result<(), StreamError> {
        require_initialized(&env)?;
        let mut stream = storage::get_stream(&env, stream_id)?;
        storage::extend_stream_ttl(&env, stream_id);

        if stream.is_canceled {
            return Err(StreamError::StreamCanceled);
        }

        stream.recipient.require_auth();

        let current_time = env.ledger().timestamp();
        let earned = compute_earned(&stream, current_time)?;
        let available = earned
            .checked_sub(stream.recipient_withdrawn)
            .ok_or(StreamError::MathOverflow)?;

        if amount > available {
            return Err(StreamError::WithdrawAmountTooHigh);
        }

        stream.recipient_withdrawn = stream
            .recipient_withdrawn
            .checked_add(amount)
            .ok_or(StreamError::MathOverflow)?;
        stream.remaining_balance = stream
            .remaining_balance
            .checked_sub(amount)
            .ok_or(StreamError::MathOverflow)?;

        storage::set_stream(&env, &stream);

        let token_client = token::Client::new(&env, &stream.token);
        token_client.transfer(&env.current_contract_address(), &stream.recipient, &amount);

        events::emit_tokens_withdrawn(
            &env,
            &stream.recipient,
            stream_id,
            amount,
            stream.remaining_balance,
        );

        Ok(())
    }

    /// Cancel a stream. Only the sender may cancel, and only if the stream is
    /// marked as cancelable. Distributes earned tokens to recipient and refunds
    /// the remainder to the sender.
    pub fn cancel(env: Env, stream_id: u64) -> Result<(), StreamError> {
        require_initialized(&env)?;
        let mut stream = storage::get_stream(&env, stream_id)?;
        storage::extend_stream_ttl(&env, stream_id);

        if stream.is_canceled {
            return Err(StreamError::StreamCanceled);
        }
        if !stream.cancelable {
            return Err(StreamError::NotCancelable);
        }

        stream.sender.require_auth();

        let current_time = env.ledger().timestamp();
        let earned = compute_earned(&stream, current_time)?;

        // Recipient payout: earned minus what they already withdrew
        let recipient_payout = earned
            .checked_sub(stream.recipient_withdrawn)
            .ok_or(StreamError::MathOverflow)?;

        // Sender refund: deposit minus total earned
        let sender_refund = stream
            .deposit_amount
            .checked_sub(earned)
            .ok_or(StreamError::MathOverflow)?;

        stream.is_canceled = true;
        stream.remaining_balance = 0;
        storage::set_stream(&env, &stream);

        let token_client = token::Client::new(&env, &stream.token);

        if recipient_payout > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.recipient,
                &recipient_payout,
            );
        }

        if sender_refund > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.sender,
                &sender_refund,
            );
        }

        events::emit_stream_canceled(&env, stream_id, sender_refund, recipient_payout);

        Ok(())
    }
}
