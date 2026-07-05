#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "bench", allow(unused_features))]
#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs)]

pub use canonical_id::CanonicalId;
pub use progress::DisplayVecProgress;
pub use progress::IdVal;
pub use progress::VecProgress;
pub use progress::VecProgressEntry;
pub use progress::VecProgressEntryData;
pub use quorum::Coherent;
pub use quorum::FindCoherent;
pub use quorum::QuorumSet;
pub use quorum_node::Node;
pub use quorum_tree::QuorumTree;
pub use quorum_tree_error::QuorumTreeError;

mod canonical_id;
mod progress;
mod quorum;
mod quorum_node;
mod quorum_tree;
mod quorum_tree_error;
mod quorum_tree_spec;
