// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert between Rust and proto representations of QuorumSet.

use crate::{
    quorum_set::{
        QuorumSet as QuorumSetProto, QuorumSetMember as QuorumSetMemberProto,
        QuorumSetMember_oneof_member,
    },
    ConversionError,
};
use mc_transaction_core::{QuorumSet, QuorumSetMember, QuorumSetMemberWrapper};
use std::convert::{Into, TryFrom, TryInto};

// mc_transaction_core::QuorumSet
impl From<&QuorumSet> for QuorumSetProto {
    fn from(qs: &QuorumSet) -> Self {
        let mut proto = QuorumSetProto::new();
        let members = qs
            .members
            .iter()
            .filter_map(|m| m.member.as_ref().map(Into::into))
            .collect();
        proto.threshold = qs.threshold;
        proto.set_members(members);
        proto
    }
}

impl TryFrom<&QuorumSetProto> for QuorumSet {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetProto) -> Result<Self, Self::Error> {
        let members = proto
            .members
            .iter()
            .map(|m| {
                Ok(QuorumSetMemberWrapper {
                    member: Some(m.try_into()?),
                })
            })
            .collect::<Result<Vec<_>, ConversionError>>()?;
        Ok(Self {
            threshold: proto.threshold,
            members,
        })
    }
}

// mc_transaction_core::QuorumSetMember
impl From<&QuorumSetMember> for QuorumSetMemberProto {
    fn from(member: &QuorumSetMember) -> QuorumSetMemberProto {
        use QuorumSetMember::*;
        let mut proto = QuorumSetMemberProto::new();
        match member {
            Node(id) => proto.set_node(id.into()),
            InnerSet(qs) => proto.set_inner_set(qs.into()),
        }
        proto
    }
}

impl TryFrom<&QuorumSetMemberProto> for QuorumSetMember {
    type Error = ConversionError;

    fn try_from(proto: &QuorumSetMemberProto) -> Result<Self, Self::Error> {
        use QuorumSetMember::*;
        use QuorumSetMember_oneof_member::*;
        match proto.member.as_ref() {
            Some(node(id)) => Ok(Node(id.try_into()?)),
            Some(inner_set(qs)) => Ok(InnerSet(qs.try_into()?)),
            None => Err(ConversionError::ObjectMissing),
        }
    }
}
