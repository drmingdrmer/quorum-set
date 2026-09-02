//! A quorum is a set of nodes a vote request or append-entries request has to contact to.
//! The most common quorum is **majority**.
//! A quorum set is a collection of quorums, e.g., the quorum set of the majority of `{a,b,c}` is
//! `{a,b}, {b,c}, {a,c}`.

mod quorum_intersection;
mod quorum_intersection_impl;
mod quorum_set;
mod quorum_set_impl;

#[cfg(test)]
mod quorum_intersection_test;
#[cfg(test)]
mod quorum_set_test;

pub use quorum_intersection::QuorumBridge;
pub use quorum_intersection::QuorumIntersection;
pub use quorum_intersection::verify_intersection;
pub use quorum_set::QuorumSet;
