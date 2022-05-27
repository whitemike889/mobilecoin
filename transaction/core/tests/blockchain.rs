use mc_account_keys::AccountKey;
use mc_transaction_core::{Block, BlockVersion};

#[test]
fn test_cumulative_txo_counts() {
    mc_util_test_helper::run_with_several_seeds(|mut rng| {
        for block_version in BlockVersion::iterator() {
            let origin = Block::new_origin_block(&[]);

            let accounts: Vec<AccountKey> =
                (0..20).map(|_i| AccountKey::random(&mut rng)).collect();
            let recipient_pub_keys = accounts
                .iter()
                .map(|account| account.default_subaddress())
                .collect::<Vec<_>>();

            let results = mc_transaction_core_test_utils::get_blocks(
                block_version,
                &recipient_pub_keys[..],
                1,
                50,
                50,
                &origin,
                &mut rng,
            );

            let mut parent = origin.clone();
            for block_data in results {
                let block = block_data.block();
                let block_txo_count = block_data.contents().outputs.len() as u64;
                assert_eq!(
                    block.cumulative_txo_count,
                    parent.cumulative_txo_count + block_txo_count
                );
                assert_eq!(block.parent_id, parent.id);
                parent = block.clone();
            }
        }
    })
}
