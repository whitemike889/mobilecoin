// Copyright (c) 2018-2022 The MobileCoin Foundation

//! The quorum set is the essential unit of trust in Stellar Consensus Protocol.
//!
//! A quorum set includes the members of the network, which a given node trusts
//! and depends on.
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    convert::TryFrom,
    hash::{Hash, Hasher},
    str::FromStr,
};
use mc_common::HashSet;
use mc_crypto_digestible::Digestible;
use mc_crypto_keys::Ed25519Public;
use prost::{Message, Oneof};
use serde::{Deserialize, Serialize};

/// A node ID.
#[derive(
    Clone, Deserialize, Digestible, Eq, Hash, Message, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct QuorumNode {
    /// The responder ID for this node.
    #[prost(string, tag = 1)]
    pub responder_id: String,

    /// The message signing key for this node.
    #[prost(required, message, tag = 2)]
    pub public_key: Ed25519Public,
}

/// A member in a [QuorumSet]. Can be either a [QuorumNode] or another
/// [QuorumSet].
#[derive(
    Clone, Deserialize, Digestible, Eq, Hash, Oneof, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum QuorumSetMember {
    /// A single trusted entity with an identity.
    #[prost(message, tag = 1)]
    Node(QuorumNode),

    /// A quorum set can also be a member of a quorum set.
    #[prost(message, tag = 2)]
    InnerSet(QuorumSet),
}

/// Prost-required wrapper for [QuorumSetMember].
#[derive(
    Clone, Deserialize, Digestible, Eq, Hash, Message, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct QuorumSetMemberWrapper {
    /// The [QuorumSetMember].
    #[prost(oneof = "QuorumSetMember", tags = "1, 2")]
    pub member: Option<QuorumSetMember>,
}

/// The quorum set defining the trusted set of peers.
#[derive(Clone, Deserialize, Digestible, Message, Ord, PartialOrd, Serialize)]
pub struct QuorumSet {
    /// Threshold (how many members do we need to reach quorum).
    #[prost(uint32, tag = 1)]
    pub threshold: u32,

    /// Members.
    #[prost(repeated, message, tag = 2)]
    pub members: Vec<QuorumSetMemberWrapper>,
}

impl PartialEq for QuorumSet {
    fn eq(&self, other: &QuorumSet) -> bool {
        if self.threshold == other.threshold && self.members.len() == other.members.len() {
            // sort before comparing
            let mut self_clone = self.clone();
            let mut other_clone = other.clone();
            self_clone.sort();
            other_clone.sort();
            return self_clone.members == other_clone.members;
        }
        false
    }
}
impl Eq for QuorumSet {}

impl Hash for QuorumSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // hash over a recursively sorted copy
        let mut qs_clone = self.clone();
        qs_clone.sort();
        qs_clone.threshold.hash(state);
        qs_clone.members.hash(state);
    }
}

impl QuorumSet {
    /// Create a new quorum set.
    pub fn new(threshold: u32, members: Vec<QuorumSetMember>) -> Self {
        Self {
            threshold,
            members: members
                .into_iter()
                .map(|member| QuorumSetMemberWrapper {
                    member: Some(member),
                })
                .collect(),
        }
    }

    /// Create a new quorum set from the given node IDs.
    pub fn new_with_node_ids(threshold: u32, node_ids: Vec<QuorumNode>) -> Self {
        Self::new(
            threshold,
            node_ids.into_iter().map(QuorumSetMember::Node).collect(),
        )
    }

    /// Create a new quorum set from the given inner sets.
    pub fn new_with_inner_sets(threshold: u32, inner_sets: Vec<Self>) -> Self {
        Self::new(
            threshold,
            inner_sets
                .into_iter()
                .map(QuorumSetMember::InnerSet)
                .collect(),
        )
    }

    /// A quorum set with no members and a threshold of 0.
    pub fn empty() -> Self {
        Self::new(0, vec![])
    }

    /// Check if a quorum set is valid.
    pub fn is_valid(&self) -> bool {
        // Must have at least `threshold` members.
        if self.threshold as usize > self.members.len() {
            return false;
        }

        // All of our inner sets must be valid.
        for member in self.members.iter() {
            if let Some(QuorumSetMember::InnerSet(qs)) = &member.member {
                if !qs.is_valid() {
                    return false;
                }
            }
        }

        // QuorumSet is valid
        true
    }

    /// Recursively sort the QS and all inner sets
    pub fn sort(&mut self) {
        for member in self.members.iter_mut() {
            if let Some(QuorumSetMember::InnerSet(qs)) = &mut member.member {
                qs.sort()
            };
        }
        // sort the members after any internal reordering!
        self.members.sort();
    }

    /// Returns a flattened set of all nodes contained in q and its nested
    /// QSets.
    pub fn nodes(&self) -> HashSet<QuorumNode> {
        let mut result = HashSet::default();
        for member in self.members.iter() {
            match &member.member {
                Some(QuorumSetMember::Node(node_id)) => {
                    result.insert(node_id.clone());
                }
                Some(QuorumSetMember::InnerSet(qs)) => {
                    result.extend(qs.nodes());
                }
                None => {}
            }
        }
        result
    }

    /// Gives the fraction of quorum slices containing the given node.
    /// It assumes that id appears in at most one QuorumSet
    /// (either the top level one or a single reachable nested one)
    /// and then only once in that QuorumSet.
    ///
    /// # Returns
    /// * (numerator, denominator) representing the node's weight.
    pub fn weight(&self, node_id: &QuorumNode) -> (u32, u32) {
        for m in self.members.iter() {
            match &m.member {
                Some(QuorumSetMember::Node(n)) => {
                    if node_id == n {
                        return (self.threshold, self.members.len() as u32);
                    }
                }
                Some(QuorumSetMember::InnerSet(qs)) => {
                    let (num2, denom2) = qs.weight(node_id);
                    if num2 > 0 {
                        return (self.threshold * num2, self.members.len() as u32 * denom2);
                    }
                }
                None => {}
            }
        }

        (0, 1)
    }
}

impl From<&mc_common::NodeID> for QuorumNode {
    fn from(src: &mc_common::NodeID) -> Self {
        QuorumNode {
            responder_id: src.responder_id.to_string(),
            public_key: src.public_key,
        }
    }
}

impl TryFrom<&QuorumNode> for mc_common::NodeID {
    type Error = mc_common::ResponderIdParseError;

    fn try_from(src: &QuorumNode) -> Result<Self, Self::Error> {
        Ok(Self {
            responder_id: mc_common::ResponderId::from_str(&src.responder_id)?,
            public_key: src.public_key,
        })
    }
}

#[cfg(test)]
mod quorum_set_tests {
    use super::*;
    use core::hash::{BuildHasher, Hash, Hasher};
    use mc_common::HasherBuilder;
    use mc_crypto_keys::Ed25519Pair;
    use mc_util_from_random::FromRandom;
    use rand::{rngs::StdRng, SeedableRng};

    fn test_node_id(id: u8) -> QuorumNode {
        let responder_id = format!("node{}.test.com:8443", id);
        let seed_bytes = [id; 32];
        let mut seeded_rng = StdRng::from_seed(seed_bytes);
        let signer_keypair = Ed25519Pair::from_random(&mut seeded_rng);
        let public_key = signer_keypair.public_key();
        QuorumNode {
            responder_id,
            public_key,
        }
    }

    fn assert_quorum_sets_equal(quorum_set_1: &QuorumSet, quorum_set_2: &QuorumSet) {
        assert_eq!(quorum_set_1, quorum_set_2);

        // qs1 == qs2 must imply hash(qs1) == hash(qs2)
        let hasher_builder = HasherBuilder::default();
        let quorum_set_1_hash = {
            let mut hasher = hasher_builder.build_hasher();
            quorum_set_1.hash(&mut hasher);
            hasher.finish()
        };
        let quorum_set_2_hash = {
            let mut hasher = hasher_builder.build_hasher();
            quorum_set_2.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(quorum_set_1_hash, quorum_set_2_hash);
    }

    #[test]
    // quorum sets should sort recursively
    fn test_quorum_set_sorting() {
        let qs = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(2)),
                        QuorumSetMember::InnerSet(QuorumSet::new_with_node_ids(
                            2,
                            vec![test_node_id(5), test_node_id(7), test_node_id(6)],
                        )),
                    ],
                )),
                QuorumSetMember::Node(test_node_id(0)),
            ],
        );
        let mut qs_sorted = qs.clone();
        qs_sorted.sort();

        assert_quorum_sets_equal(&qs, &qs_sorted);
    }

    #[test]
    // ordering of members should not matter
    fn test_quorum_set_equality_1() {
        let quorum_set_1 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(2)),
                QuorumSetMember::Node(test_node_id(3)),
            ],
        );
        let quorum_set_2 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(3)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(2)),
                QuorumSetMember::Node(test_node_id(0)),
            ],
        );

        assert_quorum_sets_equal(&quorum_set_1, &quorum_set_2);
    }

    #[test]
    // ordering of members should not matter wrt member Enum type
    fn test_quorum_set_equality_2() {
        let quorum_set_1 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(4)),
                    ],
                )),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(5)),
                        QuorumSetMember::Node(test_node_id(6)),
                        QuorumSetMember::Node(test_node_id(7)),
                    ],
                )),
            ],
        );
        let quorum_set_2 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(4)),
                    ],
                )),
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(5)),
                        QuorumSetMember::Node(test_node_id(6)),
                        QuorumSetMember::Node(test_node_id(7)),
                    ],
                )),
            ],
        );
        assert_quorum_sets_equal(&quorum_set_1, &quorum_set_2);
    }

    #[test]
    // ordering of members inside inner sets should not matter
    fn test_quorum_set_equality_3() {
        let quorum_set_1 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(4)),
                    ],
                )),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(5)),
                        QuorumSetMember::Node(test_node_id(6)),
                        QuorumSetMember::Node(test_node_id(7)),
                    ],
                )),
            ],
        );
        let quorum_set_2 = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(4)),
                        QuorumSetMember::Node(test_node_id(3)),
                    ],
                )),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(5)),
                        QuorumSetMember::Node(test_node_id(7)),
                        QuorumSetMember::Node(test_node_id(6)),
                    ],
                )),
            ],
        );
        assert_quorum_sets_equal(&quorum_set_1, &quorum_set_2);
    }

    #[test]
    fn test_is_valid() {
        // An empty quorum set is valid.
        assert!(QuorumSet::empty().is_valid());

        // A quorum set with num of members > threshold is valid.
        assert!(QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(2)),
            ],
        )
        .is_valid());

        // A quorum set with num of members == threshold is valid.
        assert!(QuorumSet::new(
            3,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(2)),
            ],
        )
        .is_valid());

        // A quorum set with num of members < threshold is invalid
        assert!(!QuorumSet::new(
            4,
            vec![
                QuorumSetMember::Node(test_node_id(0)),
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::Node(test_node_id(2)),
            ],
        )
        .is_valid());

        // A quorum set with a valid inner set is valid.
        let qs = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(2)),
                        QuorumSetMember::InnerSet(QuorumSet::new_with_node_ids(
                            2,
                            vec![test_node_id(5), test_node_id(7), test_node_id(6)],
                        )),
                    ],
                )),
                QuorumSetMember::Node(test_node_id(0)),
            ],
        );
        assert!(qs.is_valid());

        // A quorum set with an invalid inner set is invalid.
        let qs = QuorumSet::new(
            2,
            vec![
                QuorumSetMember::Node(test_node_id(1)),
                QuorumSetMember::InnerSet(QuorumSet::new(
                    2,
                    vec![
                        QuorumSetMember::Node(test_node_id(3)),
                        QuorumSetMember::Node(test_node_id(2)),
                        QuorumSetMember::InnerSet(QuorumSet::new_with_node_ids(
                            20,
                            vec![test_node_id(5), test_node_id(7), test_node_id(6)],
                        )),
                    ],
                )),
                QuorumSetMember::Node(test_node_id(0)),
            ],
        );
        assert!(!qs.is_valid());
    }

    #[test]
    fn quorum_node_conversion() {
        let node = test_node_id(1);
        let common_node =
            mc_common::NodeID::try_from(&node).expect("mc_common::NodeID from QuorumNode");
        let node2 = QuorumNode::from(&common_node);
        assert_eq!(node, node2);
    }
}
