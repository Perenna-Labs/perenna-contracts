use soroban_sdk::{Address, Env};

use crate::error::StreamError;
use crate::storage;

pub fn initialize(env: &Env, admin: &Address) -> Result<(), StreamError> {
    // The admin must authorize its own appointment. Without this, the first
    // caller of `init` could install an arbitrary address as admin.
    admin.require_auth();

    if admin == &env.current_contract_address() {
        return Err(StreamError::InvalidAdmin);
    }

    if storage::has_admin(env) {
        return Err(StreamError::AlreadyInitialized);
    }
    storage::set_admin(env, admin);
    storage::set_next_stream_id(env, 0);
    Ok(())
}
