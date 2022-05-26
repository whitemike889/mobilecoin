// Copyright (c) 2018-2022 The MobileCoin Foundation

mod mint;

pub use mc_account_keys::{AccountKey, PublicAddress, DEFAULT_SUBADDRESS_INDEX};
pub use mc_crypto_ring_signature_signer::NoKeysRingSigner;
pub use mc_fog_report_validation_test_utils::MockFogResolver;
pub use mc_transaction_core::{
    get_tx_out_shared_secret,
    onetime_keys::recover_onetime_private_key,
    ring_signature::KeyImage,
    tokens::Mob,
    tx::{Tx, TxOut, TxOutMembershipElement, TxOutMembershipHash},
    Amount, Block, BlockContents, BlockData, BlockID, BlockIndex, BlockMetadata,
    BlockMetadataContents, BlockSignature, BlockVersion, QuorumNode, QuorumSet, Token,
    VerificationReport,
};
pub use mc_util_serial::round_trip_message;
pub use mint::{
    create_mint_config_tx, create_mint_config_tx_and_signers, create_mint_tx,
    create_mint_tx_to_recipient, mint_config_tx_to_validated,
};

use mc_crypto_keys::{Ed25519Pair, RistrettoPrivate};
use mc_util_from_random::{random_bytes_vec, FromRandom};
use mc_util_test_helper::{CryptoRng, Rng, RngCore, RngType as FixedRng, SeedableRng};

/// Get blocks with custom contents to simulate conditions seen in production.
///
/// * `block_version`: the desired block version.
/// * `num_blocks`: total number of simulated blocks to create.
/// * `num_recipients`: number of randomly generated recipients.
/// * `num_tokens`: number of distinct token ids per block.
/// * `num_tx_outs_per_recipient_per_token_per_block`: number of outputs for
///   each token ID per recipient per block.
/// * `amount_per_tx_out`: amount per TxOut (per token per recipient per block).
/// * `prev_block`: Optional previous block; otherwise create an origin block.
/// * `rng`: A CSPRNG.
pub fn get_blocks<R: RngCore + CryptoRng>(
    block_version: BlockVersion,
    num_blocks: usize,
    num_recipients: usize,
    num_tokens: u64,
    num_tx_outs_per_recipient_per_token_per_block: usize,
    amount_per_tx_out: u64,
    prev_block: impl Into<Option<Block>>,
    rng: &mut R,
) -> Vec<BlockData> {
    let recipients = (0..num_recipients)
        .map(|_i| AccountKey::random(rng).default_subaddress())
        .collect::<Vec<_>>();
    get_blocks_with_recipients(
        block_version,
        num_blocks,
        &recipients,
        num_tokens,
        num_tx_outs_per_recipient_per_token_per_block,
        amount_per_tx_out,
        prev_block,
        rng,
    )
}

/// Get blocks with custom content in order to simulate conditions seen in
/// production
///
/// * `block_version`: the desired block version
/// * `num_blocks`: total number of simulated blocks to create
/// * `recipients`: recipients' public addresses
/// * `num_tokens`: number of distinct token ids per block
/// * `num_tx_outs_per_recipient_per_token_per_block`: number of outputs for
///   each token ID per recipient per block
/// * `prev_block`: Optional previous block; otherwise create an origin block.
/// * `rng`: A CSPRNG
pub fn get_blocks_with_recipients<R: RngCore + CryptoRng>(
    block_version: BlockVersion,
    num_blocks: usize,
    recipients: &[PublicAddress],
    num_tokens: u64,
    num_tx_outs_per_recipient_per_token_per_block: usize,
    amount_per_tx_out: u64,
    prev_block: impl Into<Option<Block>>,
    rng: &mut R,
) -> Vec<BlockData> {
    assert!(!recipients.is_empty());
    assert!(num_tokens > 0);
    assert!(num_tx_outs_per_recipient_per_token_per_block > 0);
    assert!(amount_per_tx_out > 0);
    assert!(block_version.mixed_transactions_are_supported() || num_tokens == 1);

    let mut blocks = Vec::with_capacity(num_blocks);
    let mut prev_block: Option<Block> = prev_block.into();

    for block_index in 0..num_blocks {
        let mut recipient_and_amount = Vec::with_capacity(
            recipients.len() * num_tokens as usize * num_tx_outs_per_recipient_per_token_per_block,
        );
        for recipient in recipients {
            for token_id in 0..num_tokens {
                for _ in 0..num_tx_outs_per_recipient_per_token_per_block {
                    recipient_and_amount.push((
                        recipient.clone(),
                        Amount::new(amount_per_tx_out, token_id.into()),
                    ));
                }
            }
        }
        let outputs = get_outputs(block_version, &recipient_and_amount, rng);

        // Non-origin blocks must have at least one key image.
        let key_images = match &prev_block {
            Some(_) => vec![KeyImage::from(block_index as u64)],
            None => vec![],
        };

        let block_contents = BlockContents {
            key_images,
            outputs,
            ..Default::default()
        };

        let block = match &prev_block {
            Some(parent) => {
                Block::new_with_parent(block_version, parent, &Default::default(), &block_contents)
            }
            None => Block::new_origin_block(&block_contents.outputs),
        };
        prev_block = Some(block.clone());

        let signature = make_block_signature(&block, rng);
        let metadata = make_block_metadata(block.id.clone(), rng);

        let block_data = BlockData::new(block, block_contents, signature, metadata);

        blocks.push(block_data);
    }
    blocks
}

/// Generate a set of outputs that "mint" coins for each recipient.
pub fn get_outputs<R: RngCore + CryptoRng>(
    block_version: BlockVersion,
    recipient_and_amount: &[(PublicAddress, Amount)],
    rng: &mut R,
) -> Vec<TxOut> {
    recipient_and_amount
        .iter()
        .map(|(recipient, amount)| {
            let mut result = TxOut::new(
                *amount,
                recipient,
                &RistrettoPrivate::from_random(rng),
                Default::default(),
            )
            .unwrap();
            if !block_version.e_memo_feature_is_supported() {
                result.e_memo = None;
            }
            result.masked_amount.masked_token_id = Default::default();
            result
        })
        .collect()
}

/// Generate a dummy txout for testing.
pub fn create_test_tx_out(rng: &mut (impl RngCore + CryptoRng)) -> TxOut {
    let account_key = AccountKey::random(rng);
    TxOut::new(
        Amount {
            value: rng.next_u64(),
            token_id: Mob::ID,
        },
        &account_key.default_subaddress(),
        &RistrettoPrivate::from_random(rng),
        Default::default(),
    )
    .unwrap()
}

pub fn make_test_node(node_id: u32) -> QuorumNode {
    make_test_node_and_signer(node_id).0
}

pub fn make_test_node_and_signer(node_id: u32) -> (QuorumNode, Ed25519Pair) {
    let mut seed_bytes = [0u8; 32];
    let node_id_bytes = node_id.to_be_bytes();
    seed_bytes[..node_id_bytes.len()].copy_from_slice(&node_id_bytes[..]);
    let mut seeded_rng = FixedRng::from_seed(seed_bytes);

    let signer_keypair = Ed25519Pair::from_random(&mut seeded_rng);
    let public_key = signer_keypair.public_key();
    (
        QuorumNode {
            responder_id: format!("node{}.test.com:8443", node_id),
            public_key,
        },
        signer_keypair,
    )
}

pub fn make_quorum_set<RNG: RngCore + CryptoRng>(num_nodes: u32, rng: &mut RNG) -> QuorumSet {
    let threshold = rng.gen_range(1..=num_nodes);
    let node_ids = (0..num_nodes).map(make_test_node).collect();
    QuorumSet::new_with_node_ids(threshold, node_ids)
}

pub fn make_verification_report<RNG: RngCore + CryptoRng>(rng: &mut RNG) -> VerificationReport {
    let sig = random_bytes_vec(42, rng).into();
    let chain_len = rng.gen_range(2..42);
    let chain = (1..=chain_len)
        .map(|n| random_bytes_vec(n as usize, rng))
        .collect();
    VerificationReport {
        sig,
        chain,
        http_body: "testing".to_owned(),
    }
}

pub fn make_block_metadata<RNG: RngCore + CryptoRng>(
    block_id: BlockID,
    rng: &mut RNG,
) -> BlockMetadata {
    let signer = Ed25519Pair::from_random(rng);
    let metadata = BlockMetadataContents::new(
        block_id,
        Some(make_quorum_set(rng.gen_range(1..=42), rng)),
        Some(make_verification_report(rng)),
    );
    BlockMetadata::from_contents_and_keypair(metadata, &signer)
        .expect("BlockMetadata::from_contents_and_keypair")
}

pub fn make_block_signature<RNG: RngCore + CryptoRng>(
    block: &Block,
    rng: &mut RNG,
) -> BlockSignature {
    let signer = Ed25519Pair::from_random(rng);
    let mut signature = BlockSignature::from_block_and_keypair(block, &signer)
        .expect("Could not create block signature from keypair");
    signature.set_signed_at(block.index);
    signature
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_transaction_core::compute_block_id;
    use mc_util_test_helper::get_seeded_rng;

    #[test]
    /// [get_blocks] should return blocks that match the configuration specified
    /// in the arguments and pass all normal consistency tests
    fn test_get_blocks_correctness() {
        let blocks = get_blocks(
            BlockVersion::MAX,
            4,
            3,
            2,
            1,
            42,
            None,
            &mut get_seeded_rng(),
        );

        // Ensure the correct amount of blocks have been created
        assert_eq!(blocks.len(), 3);

        // Ensure the origin block ID isn't a hash of another block
        let origin_block: &Block = blocks[0].block();
        assert_eq!(origin_block.parent_id.as_ref(), [0u8; 32]);
        assert_eq!(origin_block.index, 0);

        for block_data in blocks {
            let block = block_data.block();
            let contents = block_data.contents();

            // Ensure the block_id matches the id computed via the merlin transcript
            let derived_block_id = compute_block_id(
                block.version,
                &block.parent_id,
                block.index,
                block.cumulative_txo_count,
                &block.root_element,
                &block.contents_hash,
            );
            assert_eq!(derived_block_id, block.id);

            // Ensure stated block hash matches the computed hash
            assert_eq!(block.contents_hash, contents.hash());

            // Ensure the amount of transactions present matches expected amount
            assert_eq!(block.cumulative_txo_count, (block.index + 1) * 6);

            // Ensure the correct number of key images exist
            let num_key_images = if block.index == 0 { 0 } else { 1 };
            assert_eq!(contents.key_images.len(), num_key_images);
        }
    }
}
