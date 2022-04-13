// Copyright (c) 2018-2022 The MobileCoin Foundation

use crate::{BlockID, QuorumSet};
use displaydoc::Display;
use mc_attest_core::VerificationReport;
use mc_crypto_digestible::{Digestible, MerlinTranscript};
use mc_crypto_keys::{
    Ed25519Pair, Ed25519Public, Ed25519Signature, SignatureError, Signer, Verifier,
};
use prost::Message;
use serde::{Deserialize, Serialize};

/// Metadata for a block.
#[derive(Clone, Deserialize, Digestible, Display, Eq, Message, PartialEq, Serialize)]
pub struct BlockMetadata {
    /// The Block ID.
    #[prost(message, required, tag = 1)]
    block_id: BlockID,

    /// Quorum set configuration at the time of externalization.
    #[prost(message, optional, tag = 2)]
    quorum_set: Option<QuorumSet>,

    /// IAS report for the enclave which generated the signature.
    #[prost(message, optional, tag = 3)]
    verification_report: Option<VerificationReport>,
}

impl BlockMetadata {
    /// Instantiate a [BlockMetadata] with the given data.
    pub fn new(
        block_id: BlockID,
        quorum_set: Option<QuorumSet>,
        verification_report: Option<VerificationReport>,
    ) -> Self {
        Self {
            block_id,
            quorum_set,
            verification_report,
        }
    }

    /// Get the [BlockID].
    pub fn block_id(&self) -> &BlockID {
        &self.block_id
    }

    /// Get the [QuorumSet].
    pub fn quorum_set(&self) -> &Option<QuorumSet> {
        &self.quorum_set
    }

    /// Get the Attested [VerificationReport].
    pub fn verification_report(&self) -> &Option<VerificationReport> {
        &self.verification_report
    }

    fn digest(&self) -> [u8; 32] {
        self.digest32::<MerlinTranscript>(b"block_metadata")
    }
}

/// Signed metadata for a block.
#[derive(Clone, Deserialize, Digestible, Display, Eq, Message, PartialEq, Serialize)]
pub struct SignedBlockMetadata {
    /// Metadata signed by the consensus node.
    #[prost(message, required, tag = 1)]
    contents: BlockMetadata,

    /// Message signing key (signer).
    #[prost(message, required, tag = 2)]
    node_key: Ed25519Public,

    /// Signature using `node_key` over the Digestible encoding of `contents`.
    #[prost(message, required, tag = 3)]
    signature: Ed25519Signature,
}

impl SignedBlockMetadata {
    /// Instantiate a [SignedBlockMetadata] with the given data.
    pub fn new(
        contents: BlockMetadata,
        node_key: Ed25519Public,
        signature: Ed25519Signature,
    ) -> Self {
        Self {
            contents,
            node_key,
            signature,
        }
    }

    /// Instantiate a [SignedBlockMetadata] by signing the given [BlockMetadata]
    /// with the given [Ed25519Pair].
    pub fn from_contents_and_keypair(
        contents: BlockMetadata,
        key_pair: &Ed25519Pair,
    ) -> Result<Self, SignatureError> {
        let signature = key_pair.try_sign(&contents.digest())?;
        Ok(Self::new(contents, key_pair.public_key(), signature))
    }

    /// Verify that this signature is over a given block.
    pub fn verify(&self, contents: &BlockMetadata) -> Result<(), SignatureError> {
        self.node_key.verify(&contents.digest(), &self.signature)
    }

    /// Get the [BlockMetadata].
    pub fn contents(&self) -> &BlockMetadata {
        &self.contents
    }

    /// Get the signing key.
    pub fn node_key(&self) -> &Ed25519Public {
        &self.node_key
    }

    /// Get the signature.
    pub fn signature(&self) -> &Ed25519Signature {
        &self.signature
    }
}
