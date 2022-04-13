// Copyright (c) 2018-2021 The MobileCoin Foundation

use crate::{Block, BlockContents, BlockSignature, SignedBlockMetadata};
use mc_crypto_digestible::Digestible;
use prost::Message;
use serde::{Deserialize, Serialize};

/// An object that holds all data included in and associated with a block.
#[derive(Clone, Deserialize, Digestible, Eq, Message, PartialEq, Serialize)]
pub struct BlockData {
    #[prost(message, required, tag = 1)]
    block: Block,

    #[prost(message, required, tag = 2)]
    contents: BlockContents,

    #[prost(message, tag = 3)]
    signature: Option<BlockSignature>,

    #[prost(message, tag = 4)]
    metadata: Option<SignedBlockMetadata>,
}

impl BlockData {
    /// Create new block data:
    ///
    /// Arguments:
    /// `block`: The block header
    /// `contents`: The block contents
    /// `signature`: A signature over the block
    pub fn new(block: Block, contents: BlockContents, signature: Option<BlockSignature>) -> Self {
        Self {
            block,
            contents,
            signature,
            metadata: None,
        }
    }

    /// Create new block data:
    ///
    /// Arguments:
    /// `block`: The block header
    /// `contents`: The block contents
    /// `signature`: A signature over the block
    /// `metadata`: Signed metadata for the block
    /// TODO: Replace new() with this variant.
    pub fn new_with_metadata(
        block: Block,
        contents: BlockContents,
        signature: Option<BlockSignature>,
        // Allows passing `Some(qs)`, `None`, `qs`.
        metadata: impl Into<Option<SignedBlockMetadata>>,
    ) -> Self {
        Self {
            block,
            contents,
            signature,
            metadata: metadata.into(),
        }
    }

    /// Get the block.
    pub fn block(&self) -> &Block {
        &self.block
    }

    /// Get the contents.
    pub fn contents(&self) -> &BlockContents {
        &self.contents
    }

    /// Get the signature.
    pub fn signature(&self) -> &Option<BlockSignature> {
        &self.signature
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &Option<SignedBlockMetadata> {
        &self.metadata
    }
}
