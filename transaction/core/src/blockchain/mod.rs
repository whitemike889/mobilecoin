// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Blockchain data structures.

mod block;
mod block_contents;
mod block_data;
mod block_id;
mod block_metadata;
mod block_signature;
mod block_version;
mod error;
mod quorum_set;

pub use self::{
    block::{compute_block_id, Block, BlockIndex, MAX_BLOCK_VERSION},
    block_contents::{BlockContents, BlockContentsHash},
    block_data::BlockData,
    block_id::BlockID,
    block_metadata::{
        BlockMetadata, BlockMetadataContents, VerificationReport, VerificationSignature,
    },
    block_signature::BlockSignature,
    block_version::{BlockVersion, BlockVersionError, BlockVersionIterator},
    error::ConvertError,
    quorum_set::{QuorumNode, QuorumSet, QuorumSetMember, QuorumSetMemberWrapper},
};
