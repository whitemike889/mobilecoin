// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert to/from blockchain::BlockMetadata.

use crate::{blockchain, ConversionError};
use mc_transaction_core::{BlockMetadata, SignedBlockMetadata};
use std::convert::{TryFrom, TryInto};

impl From<&BlockMetadata> for blockchain::BlockMetadata {
    fn from(src: &BlockMetadata) -> Self {
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

impl TryFrom<&blockchain::BlockMetadata> for BlockMetadata {
    type Error = ConversionError;

    fn try_from(src: &blockchain::BlockMetadata) -> Result<Self, Self::Error> {
        let block_id = src.get_block_id().try_into()?;
        let quorum_set = src.quorum_set.as_ref().map(TryInto::try_into).transpose()?;
        let report = src
            .verification_report
            .as_ref()
            .map(TryInto::try_into)
            .transpose()?;
        Ok(BlockMetadata::new(block_id, quorum_set, report))
    }
}

impl From<&SignedBlockMetadata> for blockchain::SignedBlockMetadata {
    fn from(src: &SignedBlockMetadata) -> Self {
        let mut proto = Self::new();
        proto.set_contents(src.contents().into());
        proto.set_node_key(src.node_key().into());
        proto.set_signature(src.signature().into());
        proto
    }
}

impl TryFrom<&blockchain::SignedBlockMetadata> for SignedBlockMetadata {
    type Error = ConversionError;

    fn try_from(src: &blockchain::SignedBlockMetadata) -> Result<Self, Self::Error> {
        let contents = src.get_contents().try_into()?;
        let node_key = src.get_node_key().try_into()?;
        let signature = src.get_signature().try_into()?;
        Ok(SignedBlockMetadata::new(contents, node_key, signature))
    }
}
