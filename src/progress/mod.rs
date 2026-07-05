//! Progress tracks replication state, i.e., it can be considered a map of node id to already
//! replicated log id.
//!
//! The "progress" internally is a vector of scalar values.
//! The scalar value is monotonically incremental. Decreasing it is not allowed.
//! Optimization on calculating the quorum-accepted log id is done on this assumption.

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
