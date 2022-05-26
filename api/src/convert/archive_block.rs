// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert to/from blockchain::ArchiveBlock

use crate::{blockchain, ConversionError};
use mc_transaction_core::{BlockContents, BlockData, BlockMetadata, BlockSignature};
use std::convert::TryFrom;

/// Convert mc_transaction_core::BlockData --> blockchain::ArchiveBlock.
impl From<&BlockData> for blockchain::ArchiveBlock {
    fn from(src: &BlockData) -> Self {
        let mut archive_block = blockchain::ArchiveBlock::new();
        let archive_block_v1 = archive_block.mut_v1();
        archive_block_v1.set_block(src.block().into());
        archive_block_v1.set_block_contents(src.contents().into());

        if let Some(signature) = src.signature() {
            archive_block_v1.set_signature(signature.into());
        }
        if let Some(metadata) = src.metadata() {
            archive_block_v1.set_metadata(metadata.into());
        }

        archive_block
    }
}

/// Convert from blockchain::ArchiveBlock --> mc_transaction_core::BlockData
impl TryFrom<&blockchain::ArchiveBlock> for BlockData {
    type Error = ConversionError;

    fn try_from(src: &blockchain::ArchiveBlock) -> Result<Self, Self::Error> {
        if !src.has_v1() {
            return Err(ConversionError::ObjectMissing);
        }
        let archive_block_v1 = src.get_v1();

        let block = archive_block_v1.get_block().try_into()?;
        let block_contents = BlockContents::try_from(archive_block_v1.get_block_contents())?;

        let signature = archive_block_v1
            .signature
            .as_ref()
            .map(BlockSignature::try_from)
            .transpose()?;
        if let Some(signature) = signature.as_ref() {
            signature
                .verify(&block)
                .map_err(|_| ConversionError::InvalidSignature)?;
        }

        let metadata = archive_block_v1
            .metadata
            .as_ref()
            .map(BlockMetadata::try_from)
            .transpose()?;

        if block.contents_hash == block_contents.hash() && block.is_block_id_valid() {
            Ok(BlockData::new(block, block_contents, signature, metadata))
        } else {
            Err(ConversionError::InvalidContents)
        }
    }
}

/// Convert &[BlockData] -> blockchain::ArchiveBlocks
impl From<&[BlockData]> for blockchain::ArchiveBlocks {
    fn from(src: &[BlockData]) -> Self {
        let mut archive_blocks = blockchain::ArchiveBlocks::new();
        archive_blocks.set_blocks(src.iter().map(blockchain::ArchiveBlock::from).collect());
        archive_blocks
    }
}

/// Convert blockchain::ArchiveBlocks -> Vec<mc_transaction_core::BlockData>
impl TryFrom<&blockchain::ArchiveBlocks> for Vec<BlockData> {
    type Error = ConversionError;

    fn try_from(src: &blockchain::ArchiveBlocks) -> Result<Self, Self::Error> {
        let blocks_data = src
            .get_blocks()
            .iter()
            .map(BlockData::try_from)
            .collect::<Result<Vec<_>, ConversionError>>()?;

        // Ensure blocks_data form a legitimate chain of blocks.
        if blocks_data
            .iter()
            // Verify that the block ID is consistent with the cached parent ID.
            .all(|data| data.block().is_block_id_valid())
            && blocks_data
                .windows(2)
                // Verify that the cached parent ID match the previous block's ID.
                .all(|window| window[1].block().parent_id == window[0].block().id)
        {
            Ok(blocks_data)
        } else {
            Err(ConversionError::InvalidContents)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_transaction_core::{Block, BlockVersion};
    use mc_transaction_core_test_utils::get_blocks;
    use mc_util_test_helper::get_seeded_rng;

    fn generate_test_blocks_data(num_blocks: usize) -> Vec<BlockData> {
        get_blocks(
            BlockVersion::MAX,
            5,
            num_blocks,
            1,
            2,
            1 << 20,
            None,
            &mut get_seeded_rng(),
        )
    }

    #[test]
    // mc_transaction_core::BlockData <--> blockchain::ArchiveBlock
    fn test_archive_block() {
        let block_data = generate_test_blocks_data(1).pop().unwrap();

        // mc_transaction_core::BlockData -> blockchain::ArchiveBlock
        let archive_block = blockchain::ArchiveBlock::from(&block_data);
        assert_eq!(
            block_data.block(),
            &Block::try_from(archive_block.get_v1().get_block()).unwrap(),
        );
        assert_eq!(
            block_data.contents(),
            &BlockContents::try_from(archive_block.get_v1().get_block_contents()).unwrap()
        );
        assert_eq!(
            block_data.signature().clone().unwrap(),
            BlockSignature::try_from(archive_block.get_v1().get_signature()).unwrap()
        );

        // blockchain::ArchiveBlock -> mc_transaction_core::BlockData
        let block_data2 = BlockData::try_from(&archive_block).unwrap();
        assert_eq!(block_data, block_data2);
    }

    #[test]
    // Attempting to convert an ArchiveBlock with invalid signature or contents
    // should fail.
    fn try_from_blockchain_archive_block_rejects_invalid() {
        let block_data = generate_test_blocks_data(1).pop().unwrap();

        // ArchiveBlock with invalid signature cannot be converted back to BlockData
        let mut archive_block = blockchain::ArchiveBlock::from(&block_data);
        archive_block
            .mut_v1()
            .mut_signature()
            .mut_signature()
            .mut_data()[0] += 1;
        assert_eq!(
            BlockData::try_from(&archive_block),
            Err(ConversionError::InvalidSignature)
        );

        // ArchiveBlock with invalid contents cannot be converted back to BlockData
        let mut archive_block = blockchain::ArchiveBlock::from(&block_data);
        archive_block
            .mut_v1()
            .mut_block_contents()
            .mut_key_images()
            .clear();
        assert_eq!(
            BlockData::try_from(&archive_block),
            Err(ConversionError::InvalidContents)
        );
    }

    #[test]
    // Vec<mc_transaction_core::BlockData> <--> blockchain::ArchiveBlocks
    fn test_archive_blocks() {
        let blocks_data = generate_test_blocks_data(10);

        // Vec<mc_transaction_core::BlockData> -> blockchain::ArchiveBlocks
        let archive_blocks = blockchain::ArchiveBlocks::from(&blocks_data[..]);
        for (i, block_data) in blocks_data.iter().enumerate() {
            let archive_block = &archive_blocks.get_blocks()[i];
            assert_eq!(
                block_data.block(),
                &Block::try_from(archive_block.get_v1().get_block()).unwrap(),
            );
            assert_eq!(
                block_data.contents(),
                &BlockContents::try_from(archive_block.get_v1().get_block_contents()).unwrap()
            );
            assert_eq!(
                block_data.signature().clone().unwrap(),
                BlockSignature::try_from(archive_block.get_v1().get_signature()).unwrap()
            );
        }

        // blockchain::ArchiveBlocks -> Vec<mc_transaction_core::BlockData>
        let blocks_data2 = Vec::<BlockData>::try_from(&archive_blocks).unwrap();
        assert_eq!(blocks_data, blocks_data2);
    }

    #[test]
    // blockchain::ArchiveBlocks -> Vec<mc_transaction_core::BlockData> should fail
    // if the blocks to not form a chain.
    fn test_try_from_blockchain_archive_blocks_rejects_invalid() {
        let blocks_data = generate_test_blocks_data(10);
        let mut archive_blocks = blockchain::ArchiveBlocks::from(&blocks_data[..]);
        archive_blocks.mut_blocks().remove(5);

        assert_eq!(
            Vec::<BlockData>::try_from(&archive_blocks),
            Err(ConversionError::InvalidContents),
        );
    }
}
