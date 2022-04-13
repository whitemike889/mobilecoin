// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert between Rust and proto representations of QuorumSet.

use crate::{
    quorum_set::{
        QuorumSet as QuorumSetProto, QuorumSetMember as QuorumSetMemberProto,
        QuorumSetMember_oneof_member,
    },
    ConversionError,
};
use mc_common::NodeID;
use mc_consensus_scp as scp;
use mc_transaction_core as txn;
use std::convert::{Into, TryFrom, TryInto};

// mc_consensus_scp::QuorumSet
impl From<&scp::QuorumSetMember<NodeID>> for QuorumSetMemberProto {
    fn from(member: &scp::QuorumSetMember<NodeID>) -> QuorumSetMemberProto {
        use scp::QuorumSetMember::*;
        let mut proto = QuorumSetMemberProto::new();
        match member {
            Node(id) => proto.set_node(id.into()),
            InnerSet(qs) => proto.set_inner_set(qs.into()),
        }
        proto
    }
}

impl From<&scp::QuorumSet> for QuorumSetProto {
    fn from(qs: &scp::QuorumSet) -> QuorumSetProto {
        let mut proto = QuorumSetProto::new();
        proto.threshold = qs.threshold;
        proto.set_members(qs.members.iter().map(Into::into).collect());
        proto
    }
}

impl TryFrom<&QuorumSetMemberProto> for scp::QuorumSetMember<NodeID> {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetMemberProto) -> Result<Self, Self::Error> {
        use scp::QuorumSetMember::*;
        use QuorumSetMember_oneof_member::*;
        match proto.member.as_ref() {
            Some(node(id)) => Ok(Node(id.try_into()?)),
            Some(inner_set(qs)) => Ok(InnerSet(qs.try_into()?)),
            None => Err(ConversionError::ObjectMissing),
        }
    }
}

impl TryFrom<&QuorumSetProto> for scp::QuorumSet {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetProto) -> Result<Self, Self::Error> {
        let members = proto
            .members
            .iter()
            .map(TryFrom::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            threshold: proto.threshold,
            members,
        })
    }
}

// mc_transaction_core::QuorumSet
impl From<&txn::QuorumSetMember> for QuorumSetMemberProto {
    fn from(member: &txn::QuorumSetMember) -> QuorumSetMemberProto {
        use txn::QuorumSetMember::*;
        let mut proto = QuorumSetMemberProto::new();
        match member {
            Node(id) => proto.set_node(id.into()),
            InnerSet(qs) => proto.set_inner_set(qs.into()),
        }
        proto
    }
}

impl From<&txn::QuorumSetMemberWrapper> for QuorumSetMemberProto {
    fn from(src: &txn::QuorumSetMemberWrapper) -> Self {
        match &src.member {
            Some(member) => member.into(),
            None => QuorumSetMemberProto::new(),
        }
    }
}

impl From<&txn::QuorumSet> for QuorumSetProto {
    fn from(qs: &txn::QuorumSet) -> QuorumSetProto {
        let mut proto = QuorumSetProto::new();
        proto.threshold = qs.threshold;
        proto.set_members(qs.members.iter().map(Into::into).collect());
        proto
    }
}

impl TryFrom<&QuorumSetMember_oneof_member> for txn::QuorumSetMember {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetMember_oneof_member) -> Result<Self, Self::Error> {
        use txn::QuorumSetMember::*;
        use QuorumSetMember_oneof_member::*;
        match proto {
            node(n) => Ok(Node(n.try_into()?)),
            inner_set(qs) => Ok(InnerSet(qs.try_into()?)),
        }
    }
}

impl TryFrom<&QuorumSetMemberProto> for txn::QuorumSetMemberWrapper {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetMemberProto) -> Result<Self, Self::Error> {
        let member = proto.member.as_ref().map(TryFrom::try_from).transpose()?;
        Ok(Self { member })
    }
}

impl TryFrom<&QuorumSetProto> for txn::QuorumSet {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetProto) -> Result<Self, Self::Error> {
        let members = proto
            .members
            .iter()
            .map(TryFrom::try_from)
            .collect::<Result<Vec<_>, Self::Error>>()?;
        Ok(Self {
            threshold: proto.threshold,
            members,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_consensus_scp::test_utils::three_node_dense_graph;

    #[test]
    fn test_roundtrip() {
        let set = three_node_dense_graph().0 .1;
        let proto = QuorumSetProto::from(&set);
        let set2 = scp::QuorumSet::try_from(&proto).expect("scp::QuorumSet from proto");
        assert_eq!(set, set2);
        assert!(set2.is_valid());

        let proto2 = QuorumSetProto::from(&set2);
        assert_eq!(proto, proto2);

        let set3 =
            txn::QuorumSet::try_from(&proto).expect("mc_transaction_core::QuorumSet from proto");
        let proto3 = QuorumSetProto::from(&set3);
        assert_eq!(proto2, proto3);
    }
}
