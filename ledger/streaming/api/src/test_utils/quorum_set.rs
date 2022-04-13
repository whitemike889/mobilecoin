// Copyright (c) 2018-2022 The MobileCoin Foundation

//! QuorumSet helpers for tests.

use mc_transaction_core::{QuorumNode, QuorumSet, QuorumSetMember};

/// Creates NodeID from integer for testing.
pub fn test_node_id(id: u32) -> QuorumNode {
    let scp_node = mc_consensus_scp::test_utils::test_node_id(id);
    (&scp_node).into()
}

/// Create a QuorumSet for tests.
pub fn make_quorum_set() -> QuorumSet {
    let qs = QuorumSet::new(
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
    assert!(qs.is_valid());
    qs
}
