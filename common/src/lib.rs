// Copyright (c) 2018-2021 The MobileCoin Foundation

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![warn(unused_extern_crates)]
extern crate alloc;

pub mod hash;
pub mod hasher_builder;
pub mod logger;
pub mod lru;
pub mod node_id;
pub mod responder_id;
pub mod time;

pub use crate::{
    hash::*,
    hasher_builder::HasherBuilder,
    lru::LruCache,
    node_id::{NodeID, NodeIDError},
    responder_id::{ResponderId, ResponderIdParseError},
};

// Loggers
cfg_if::cfg_if! {
    if #[cfg(feature = "loggers")] {
        mod panic_handler;

        pub mod sentry;

        pub use crate::panic_handler::setup_panic_handler;
    }
}
