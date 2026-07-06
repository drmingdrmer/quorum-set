#![doc = include_str!("../README.md")]
#![cfg_attr(feature = "bench", allow(unused_features))]
#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs)]

pub use progress::DisplayVecProgress;
pub use progress::IdVal;
pub use progress::VecProgress;
pub use progress::VecProgressEntry;
pub use progress::VecProgressEntryData;
pub use quorum::Coherent;
pub use quorum::FindCoherent;
pub use quorum::QuorumSet;
pub use tree::CanonicalId;
pub use tree::Node;
pub use tree::QuorumTree;
pub use tree::QuorumTreeError;

mod progress;
mod quorum;
mod tree;
