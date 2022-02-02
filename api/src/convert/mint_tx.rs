// Copyright (c) 2018-2022 The MobileCoin Foundation

//! Convert to/from external::MintTx.

use crate::{convert::ConversionError, external};
use mc_transaction_core::mint;
use std::convert::TryFrom;

/// Convert mc_transaction_core::mint::MintTx to external::MintTx.
impl From<&mint::MintTx> for external::MintTx {
    fn from(source: &mint::MintTx) -> Self {
        let mut dst = external::MintTx::new();

        dst.set_amount(source.amount);
        dst.set_token_id(source.token_id);
        dst.set_view_public_key(external::CompressedRistretto::from(&source.view_public_key));
        dst.set_spend_public_key(external::CompressedRistretto::from(
            &source.spend_public_key,
        ));
        dst.set_tombstone_block(source.tombstone_block);
        dst
    }
}

/// Convert external::MintTx to mc_transaction_core::mint::MintTx.
impl TryFrom<&external::MintTx> for mint::MintTx {
    type Error = ConversionError;

    fn try_from(source: &external::MintTx) -> Result<Self, Self::Error> {
        let spend_public_key = source
            .spend_public_key
            .as_ref()
            .ok_or(mc_crypto_keys::KeyError::LengthMismatch(0, 32))
            .and_then(|key| mc_crypto_keys::RistrettoPublic::try_from(&key.data[..]))?;

        let view_public_key = source
            .view_public_key
            .as_ref()
            .ok_or(mc_crypto_keys::KeyError::LengthMismatch(0, 32))
            .and_then(|key| mc_crypto_keys::RistrettoPublic::try_from(&key.data[..]))?;

        let amount = source.amount;
        let token_id = source.token_id;
        let tombstone_block = source.tombstone_block;

        Ok(mint::MintTx {
            amount,
            token_id,
            view_public_key,
            spend_public_key,
            tombstone_block,
        })
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
