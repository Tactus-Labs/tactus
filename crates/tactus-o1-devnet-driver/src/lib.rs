//! Experiment A devnet driver — see `specs/EXPERIMENT_A_DESIGN.md` §8.
//!
//! Talks to a local CKB devnet node over JSON-RPC, builds and signs
//! transactions (molecule encoding hand-rolled in [`molecule`]), deploys the
//! OrderingHead type script and replays the A1 scenario arms on-chain.

pub mod molecule;
pub mod rpc;
pub mod tx;
