use mc_transaction_core::BlockVersion;
use mc_transaction_core_test_utils::get_blocks;

#[test]
fn test_cumulative_txo_counts() {
    mc_util_test_helper::run_with_several_seeds(|mut rng| {
        for block_version in BlockVersion::iterator() {
            let num_tokens = if block_version.mixed_transactions_are_supported() {
                3
            } else {
                1
            };
            let results = get_blocks(block_version, 50, 20, num_tokens, 5, 50, None, &mut rng);

            let mut parent = results[0].block().clone();
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
