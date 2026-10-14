use soroban_sdk::{Address, Env};

use crate::error::StreamError;
use crate::types::{DataKey, Stream};

/// Shared TTL boundaries for both instance and persistent entries (~30 days at 5s/ledger).
const TTL_THRESHOLD: u32 = 17_280;
const TTL_EXTEND: u32 = 518_400;

// Instance entries share the contract instance's TTL and are not extended
// automatically, so every instance write must bump them explicitly. When the
// instance TTL reaches zero the contract is archived and unusable until
// restored.

/// Extends the TTL of the contract instance and all instance entries
/// (`Admin`, `NextStreamId`).
pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND);
}

// --- Admin Storage ---

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    extend_instance_ttl(env);
}

// --- NextStreamId Storage ---

pub fn get_next_stream_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextStreamId)
        .unwrap_or(0)
}

pub fn set_next_stream_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextStreamId, &id);
    extend_instance_ttl(env);
}

// --- Stream Storage ---

pub fn get_stream(env: &Env, stream_id: u64) -> Result<Stream, StreamError> {
    let key = DataKey::Stream(stream_id);
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(StreamError::StreamNotFound)
}

pub fn set_stream(env: &Env, stream: &Stream) {
    let key = DataKey::Stream(stream.id);
    env.storage().persistent().set(&key, stream);
    extend_stream_ttl(env, stream.id);
}

pub fn extend_stream_ttl(env: &Env, stream_id: u64) {
    let key = DataKey::Stream(stream_id);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
}
