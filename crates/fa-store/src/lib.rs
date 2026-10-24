//! fa-store: Postgres access (migrations, RLS-scoped transactions, repositories, envelope crypto).
//!
//! All data access goes through a [`Tx`] from [`Store::tx_for`] (household-scoped, RLS enforced)
//! or [`Store::tx_system`] (cross-household paths: login lookup, device token, workers).

mod error;
pub mod repo;
pub mod secret;
mod store;

pub use error::{Result, StoreError};
pub use repo::*;
pub use secret::{MasterKey, SealedSecret};
pub use store::{Store, Tx};

/// An entity together with its optimistic-concurrency version.
#[derive(Debug, Clone)]
pub struct Versioned<T> {
    pub value: T,
    pub version: i32,
}
