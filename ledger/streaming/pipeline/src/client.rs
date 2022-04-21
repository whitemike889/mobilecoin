// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Helpers for client pipelines.

use grpcio::Environment;
use mc_common::logger::Logger;
use mc_ledger_db::Ledger;
use mc_ledger_streaming_api::Result;
use mc_ledger_streaming_client::{
    BackfillingStream, BlockchainUrl, GrpcBlockSource, HttpBlockFetcher,
};
use mc_util_uri::ConnectionUri;
use std::sync::Arc;

/// Construct a gRPC-to-ledger-DB client pipeline.
pub fn consensus_client<L: Ledger, U: ConnectionUri>(
    grpc_uri: &U,
    client_env: Arc<Environment>,
    fetch_url: BlockchainUrl,
    _ledger: Option<L>,
    logger: Logger,
) -> Result<BackfillingStream<GrpcBlockSource, HttpBlockFetcher>> {
    let grpc_source = GrpcBlockSource::new(grpc_uri, client_env, logger.clone());
    let fetcher = HttpBlockFetcher::new(fetch_url, logger.clone())?;

    let backfilling_source = BackfillingStream::new(grpc_source, fetcher, logger);
    Ok(backfilling_source)
}
