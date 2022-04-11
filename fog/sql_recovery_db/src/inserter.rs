//! Creates test data for the RecoveryDB that Fog View pulls from.

use crate::{models, schema, proto_types::ProtoIngestedBlockData};
use crate::diesel::RunQueryDsl;
use diesel::{pg::PgConnection, r2d2::{ConnectionManager, Pool}};
use mc_crypto_keys::{CompressedRistrettoPublic, RistrettoPublic};
use mc_fog_kex_rng::KexRngPubkey;
use mc_fog_test_infra::db_tests::random_kex_rng_pubkey;
use mc_fog_types::ETxOutRecord;
use mc_util_from_random::FromRandom;
use prost::Message;
use rand::{rngs::StdRng, SeedableRng};
use std::time::{SystemTime, UNIX_EPOCH};


const NUMBER_OF_TX_OUTS: i64 = 130_000_000;
const NUMBER_OF_TX_OUTS_PER_BLOCK: i64 = 10_000;

/// Inserts test data into the recovery db.
pub fn insert_test_data(db_pool: &Pool<ConnectionManager<PgConnection>>) {
    let start_block: i64 = 0;
    let pubkey_expiry = calculate_number_of_blocks() + 10;
    let mut rng: StdRng = SeedableRng::from_seed([123u8; 32]);
    let ingress_public_key: CompressedRistrettoPublic =
        CompressedRistrettoPublic::from(RistrettoPublic::from_random(&mut rng));
    let egress_public_key: KexRngPubkey = random_kex_rng_pubkey(&mut rng);

    println!("Inserting ingress keys");
    insert_ingress_key(&db_pool, &ingress_public_key, start_block, pubkey_expiry);
    println!("Inserted ingress keys");
    println!("Inserting ingest_invocation");
    let ingest_invocation_id = insert_ingest_invocation(&db_pool, &ingress_public_key, &egress_public_key, start_block);
    println!("Inserted ingest_invocation");
    println!("Inserting ingested blocks");
    insert_ingested_blocks_data(&db_pool, &ingress_public_key, ingest_invocation_id);
    println!("Inserted ingested blocks");
}

fn insert_ingest_invocation(
    db_pool: &Pool<ConnectionManager<PgConnection>>,
    ingress_public_key: &CompressedRistrettoPublic,
    egress_public_key: &KexRngPubkey,
    start_block: i64) -> i64 {
    let conn = db_pool.get().expect("Couldn't get connection");
    let now =
        diesel::select(diesel::dsl::now).get_result::<chrono::NaiveDateTime>(&conn).expect("Couldn't get time");

    let obj = models::NewIngestInvocation {
        ingress_public_key: (*ingress_public_key).into(),
        egress_public_key: egress_public_key.public_key.clone(),
        last_active_at: now,
        start_block: start_block as i64,
        decommissioned: false,
        rng_version: egress_public_key.version as i32,
    };

    let inserted_obj: models::IngestInvocation =
        diesel::insert_into(schema::ingest_invocations::table)
        .values(&obj)
        .get_result(&conn).expect("Couldn't insert ingest invocation");

    inserted_obj.id
}

fn insert_ingress_key(
    db_pool: &Pool<ConnectionManager<PgConnection>>,
    ingress_public_key: &CompressedRistrettoPublic,
    start_block: i64,
    pubkey_expiry: i64,
) {
    let conn = db_pool.get().expect("Couldn't get connection");
    let obj = models::NewIngressKey {
        ingress_public_key: (*ingress_public_key).into(),
        start_block,
        pubkey_expiry,
        retired: false,
        lost: false,
    };

    let _inserted_row_count = diesel::insert_into(schema::ingress_keys::table)
        .values(&obj)
        .on_conflict_do_nothing()
        .execute(&conn).expect("couldn't insert ingress key");
}

fn calculate_number_of_blocks() -> i64 {
    NUMBER_OF_TX_OUTS / NUMBER_OF_TX_OUTS_PER_BLOCK + 1
}

fn insert_ingested_blocks_data(db_pool: &Pool<ConnectionManager<PgConnection>>, ingress_public_key: &CompressedRistrettoPublic, ingest_invocation_id: i64) {
    let conn = db_pool.get().expect("Couldn't get connection");
    let ingress_public_key_bytes = ingress_public_key.as_bytes();
    let proto_bytes = create_proto_ingested_block_data_bytes();

    let number_of_blocks = calculate_number_of_blocks();
    for i in 0..number_of_blocks {
        let cumulative_txo_count = i * NUMBER_OF_TX_OUTS_PER_BLOCK;
        let block_signature_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Couldn't get seconds from UNIX_EPOCH")
            .as_secs();
        let new_ingested_block = models::NewIngestedBlock {
            ingress_public_key: ingress_public_key_bytes.to_vec(),
            ingest_invocation_id,
            block_number: i as i64,
            cumulative_txo_count: cumulative_txo_count as i64,
            block_signature_timestamp: block_signature_timestamp as i64,
            proto_ingested_block_data: proto_bytes.clone(),
        };

        println!("Insert the {} block into the db", i);
        diesel::insert_into(schema::ingested_blocks::table)
            .values(&new_ingested_block)
            .execute(&conn).expect("Couldn't insert ingested block");
    }
}

fn create_proto_ingested_block_data_bytes() -> Vec<u8> {
    let e_tx_out_records_per_block = create_e_tx_out_records_per_block();
    let proto_bytes = {
        let proto_ingested_block_data = ProtoIngestedBlockData {
            e_tx_out_records: e_tx_out_records_per_block.to_vec(),
        };
        let mut bytes =
            Vec::<u8>::with_capacity(proto_ingested_block_data.encoded_len());
        proto_ingested_block_data.encode(&mut bytes).expect("Couldn't encode proto bytes");
        bytes
    };

    proto_bytes
}

fn create_e_tx_out_records_per_block() -> Vec<ETxOutRecord> {
    let mut e_tx_out_records_per_block = Vec::new();
    for _ in 0..NUMBER_OF_TX_OUTS_PER_BLOCK {
        let search_key: Vec<u8> = vec![1; 16];
        let payload: Vec<u8> = vec![2; 232];
        let e_tx_out_record = ETxOutRecord {
            search_key,
            payload,
        };
        e_tx_out_records_per_block.push(e_tx_out_record);
    }

    e_tx_out_records_per_block
}
