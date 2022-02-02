// Copyright (c) 2018-2022 The MobileCoin Foundation

use mc_crypto_digestible::Digestible;
use mc_crypto_keys::RistrettoPublic;
use prost::Message;
use serde::{Deserialize, Serialize};

/// TODO
#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Digestible, Message,
)]
pub struct MintTx {
    #[prost(uint64, tag = "1")]
    pub amount: u64,

    #[prost(uint32, tag = "2")]
    pub token_id: u32,

    /// The recipient's public subaddress view key 'C'.
    // Note that we are not using PublicAddress here since right now it does not implement
    // Serialize/Deserialize and the mc_account_keys crate does not depend on serde.
    #[prost(message, required, tag = "3")]
    pub view_public_key: RistrettoPublic,

    /// The recipient's public subaddress spend key `D`.
    // Note that we are not using PublicAddress here since right now it does not implement
    // Serialize/Deserialize and the mc_account_keys crate does not depend on serde.
    #[prost(message, required, tag = "4")]
    pub spend_public_key: RistrettoPublic,

    #[prost(uint64, tag = "5")]
    pub tombstone_block: u64,
}
