//! Progress tracking over a [`QuorumSet`](crate::QuorumSet).
//!
//! [`VecProgress`] stores per-node progress values and keeps the greatest value
//! accepted by the configured quorum set. In Raft terms, it can be viewed as a
//! map from node ID to the latest replicated log ID.
//!
//! Normal progress updates are monotonic: an entry's progress may stay the same
//! or increase. `VecProgress` uses that rule to avoid unnecessary quorum
//! recalculation and sorting. The explicit reset API can move one entry backward
//! without lowering the recorded quorum-accepted value.
//!
//! This module is designed for small consensus memberships where a compact
//! vector is simpler than indexed storage.

#[cfg(feature = "bench")]
#[cfg(test)]
mod bench;
mod display_vec_progress;
mod id_val;
mod progress_stats;
mod vec_progress;
mod vec_progress_entry;

pub use display_vec_progress::DisplayVecProgress;
pub use id_val::IdVal;
pub use vec_progress::VecProgress;
pub use vec_progress_entry::VecProgressEntry;
pub use vec_progress_entry::VecProgressEntryData;
