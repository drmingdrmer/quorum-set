//! A quorum is a set of nodes a vote request or append-entries request has to contact to.
//! The most common quorum is **majority**.
//! A quorum set is a collection of quorums, e.g., the quorum set of the majority of `{a,b,c}` is
//! `{a,b}, {b,c}, {a,c}`.

mod both_quorums;
mod explicit_quorum_set;
mod grid_quorum_set;
mod grid_quorum_set_error;
mod hierarchical_quorum_set;
mod hierarchical_quorum_set_error;
mod joint;
mod quorum_bridge;
mod quorum_bridge_impl;
mod quorum_intersection;
mod quorum_intersection_impl;
mod quorum_set;
mod quorum_set_impl;
mod read_or_both;
mod threshold_quorum_set;
mod weighted_quorum_set;
mod weighted_quorum_set_error;

#[cfg(test)]
mod explicit_quorum_set_test;
#[cfg(test)]
mod grid_quorum_set_test;
#[cfg(test)]
mod hierarchical_quorum_set_test;
#[cfg(test)]
mod joint_test;
#[cfg(test)]
mod quorum_bridge_test;
#[cfg(test)]
mod quorum_intersection_test;
#[cfg(test)]
mod quorum_set_test;
#[cfg(test)]
mod testing;
#[cfg(test)]
mod threshold_quorum_set_test;
#[cfg(test)]
mod weighted_quorum_set_test;

pub use both_quorums::BothQuorums;
pub use explicit_quorum_set::ExplicitQuorumSet;
pub use grid_quorum_set::GridQuorumSet;
pub use grid_quorum_set::GridRule;
pub use grid_quorum_set_error::GridQuorumSetError;
pub use hierarchical_quorum_set::Group;
pub use hierarchical_quorum_set::HierarchicalQuorumSet;
pub use hierarchical_quorum_set_error::HierarchicalQuorumSetError;
pub use joint::Joint;
pub use quorum_bridge::QuorumBridge;
pub use quorum_intersection::QuorumIntersection;
pub use quorum_intersection::verify_intersection;
pub use quorum_set::QuorumSet;
pub use read_or_both::ReadOrBoth;
pub use threshold_quorum_set::ThresholdQuorumSet;
pub use weighted_quorum_set::WeightedQuorumSet;
pub use weighted_quorum_set_error::WeightedQuorumSetError;
