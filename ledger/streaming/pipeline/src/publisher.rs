// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Helpers for server pipelines.

use futures::{Stream, StreamExt};
use grpcio::Environment;
use mc_common::logger::Logger;
use mc_ledger_db::Ledger;
use mc_ledger_streaming_api::{Error, Result};
use mc_ledger_streaming_client::GrpcBlockSource;
#[cfg(feature = "publisher_local")]
use mc_ledger_streaming_publisher::LocalFileProtoWriter;
use mc_ledger_streaming_publisher::{ArchiveBlockSink, GrpcServerSink};
#[cfg(feature = "publisher_s3")]
use mc_ledger_streaming_publisher::{S3ClientProtoWriter, S3Config};
use mc_util_uri::ConnectionUri;
use std::{path::PathBuf, sync::Arc};

pub struct LedgerToGrpc<L: Ledger> {
    pub source: L,
    pub sink: GrpcServerSink,
}

impl<L: Ledger> LedgerToGrpc<L> {
    pub fn new(source: L, logger: Logger) -> Self {
        let sink = GrpcServerSink::new(logger);
        Self { source, sink }
    }
}

#[cfg(feature = "publisher_local")]
pub struct GrpcToLocal<L: Ledger> {
    pub source: GrpcBlockSource,
    pub sink: ArchiveBlockSink<LocalFileProtoWriter, L>,
}

#[cfg(feature = "publisher_local")]
impl<L: Ledger> GrpcToLocal<L> {
    pub fn new(
        grpc_uri: &impl ConnectionUri,
        env: Arc<Environment>,
        ledger: L,
        path: PathBuf,
        logger: Logger,
    ) -> Self {
        let source = GrpcBlockSource::new(grpc_uri, env, logger.clone());
        let sink = ArchiveBlockSink::new_local(path, ledger, logger);
        Self { source, sink }
    }
}

#[cfg(feature = "publisher_s3")]
pub struct GrpcToS3<L: Ledger> {
    pub source: GrpcBlockSource,
    pub sink: ArchiveBlockSink<S3ClientProtoWriter, L>,
}

#[cfg(feature = "publisher_s3")]
impl<L: Ledger> GrpcToS3<L> {
    pub fn new(
        grpc_uri: &impl ConnectionUri,
        env: Arc<Environment>,
        config: S3Config,
        ledger: L,
        path: PathBuf,
        logger: Logger,
    ) -> Self {
        let source = GrpcBlockSource::new(grpc_uri, env, logger.clone());
        let sink = ArchiveBlockSink::new_s3_config(config, path, ledger, logger);
        Self { source, sink }
    }
}

/// A pipeline that subscribes to the given URI, and repeats any
/// [ArchiveBlock]s it receives from that URI over its own gRPC server.
pub struct GrpcRepeater {
    pub source: GrpcBlockSource,
    pub sink: GrpcServerSink,
}

impl GrpcRepeater {
    pub fn new(grpc_uri: &impl ConnectionUri, env: Arc<Environment>, logger: Logger) -> Self {
        let source = GrpcBlockSource::new(grpc_uri, env, logger.clone());
        let sink = GrpcServerSink::new(logger);
        Self { source, sink }
    }

    /// The returned value is a `Stream` where the `Output` type is
    /// `Result<()>`; it is executed entirely for its side effects, while
    /// propagating errors back to the caller.
    pub fn subscribe_and_repeat(
        &self,
        starting_height: u64,
    ) -> Result<impl Stream<Item = Result<()>>> {
        let stream = self
            .source
            .subscribe(starting_height)?
            .map(|result| result.map_err(Error::from));
        Ok(self.sink.consume_protos(stream))
    }

    /// Create a [grpcio::Server] with a [LedgerUpdates] service backed by
    /// this pipeline.
    pub fn create_server(
        &self,
        uri: &impl ConnectionUri,
        env: Arc<grpcio::Environment>,
    ) -> grpcio::Result<grpcio::Server> {
        self.sink.create_server(uri, env)
    }

    /// Helper to create a local server.
    #[cfg(any(test, feature = "test_utils"))]
    pub fn create_local_server(
        &self,
        env: Arc<grpcio::Environment>,
    ) -> (grpcio::Server, mc_util_uri::ConsensusPeerUri) {
        self.sink.create_local_server(env)
    }
}
