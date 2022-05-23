// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert to/from blockchain::BlockMetadataContents.

use crate::{blockchain, ConversionError};
use mc_transaction_core::{BlockMetadata, BlockMetadataContents};
use std::convert::{TryFrom, TryInto};

impl From<&BlockMetadataContents> for blockchain::BlockMetadataContents {
    fn from(src: &BlockMetadataContents) -> Self {
        let mut proto = Self::new();
        proto.set_block_id(src.block_id().into());
        if let Some(qs) = src.quorum_set() {
            proto.set_quorum_set(qs.into());
        }
        if let Some(avr) = src.verification_report() {
            proto.set_verification_report(avr.into());
        }
        proto
    }
}

impl TryFrom<&blockchain::BlockMetadataContents> for BlockMetadataContents {
    type Error = ConversionError;

    fn try_from(src: &blockchain::BlockMetadataContents) -> Result<Self, Self::Error> {
        let block_id = src.get_block_id().try_into()?;
        let quorum_set = src.quorum_set.as_ref().map(TryInto::try_into).transpose()?;
        let report = src
            .verification_report
            .as_ref()
            .map(TryInto::try_into)
            .transpose()?;
        Ok(BlockMetadataContents::new(block_id, quorum_set, report))
    }
}

impl From<&BlockMetadata> for blockchain::BlockMetadata {
    fn from(src: &BlockMetadata) -> Self {
        let mut proto = Self::new();
        proto.set_contents(src.contents().into());
        proto.set_node_key(src.node_key().into());
        proto.set_digest_signature(src.digest_signature().into());
        proto
    }
}

impl TryFrom<&blockchain::BlockMetadata> for BlockMetadata {
    type Error = ConversionError;

    fn try_from(src: &blockchain::BlockMetadata) -> Result<Self, Self::Error> {
        let contents = src.get_contents().try_into()?;
        let node_key = src.get_node_key().try_into()?;
        let digest_signature = src.get_digest_signature().try_into()?;
        Ok(BlockMetadata::new(contents, node_key, digest_signature))
    }
}
