// Copyright (c) 2018-2022 The MobileCoin Foundation

//! End-to-end test of client + publisher.

use mc_common::logger::{test_with_logger, Logger};
use mc_ledger_db::test_utils::get_mock_ledger;
use mc_ledger_streaming_client::BlockchainUrl;
use mc_ledger_streaming_pipeline::{consensus_client, LedgerToGrpc};
use std::sync::Arc;
use tempdir::TempDir;
use url::Url;

#[test_with_logger]
fn chain(logger: Logger) {
    let ledger = get_mock_ledger(100);

    let _root = TempDir::new("ledger_streaming")
        .expect("tempdir")
        .into_path();

    let server_pipeline = LedgerToGrpc::new(ledger.clone(), logger.clone());
    let server_env = Arc::new(
        grpcio::EnvBuilder::new()
            .name_prefix("ledger_streaming_server".to_string())
            .build(),
    );
    let (_server, uri) = server_pipeline.sink.create_local_server(server_env);

    // Build gRPC env for initiating peer connections
    let client_env = Arc::new(
        grpcio::EnvBuilder::new()
            .name_prefix("ledger_streaming_client".to_string())
            .build(),
    );
    let url = BlockchainUrl::new(Url::parse("https://example.com/TODO").expect("Url::parse"))
        .expect("BlockchainUrl");
    let _client_pipeline = consensus_client(&uri, client_env, url, Some(ledger), logger.clone())
        .expect("consensus_client");
}
