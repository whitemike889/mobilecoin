// Copyright (c) 2018-2022 The MobileCoin Foundation

#![no_std]

extern crate alloc;
use alloc::vec::Vec;

pub extern crate prost;

pub use prost::{DecodeError, EncodeError, Message};

// We put a new-type around serde_cbor::Error in `mod decode` and `mod encode`,
// because this keeps us compatible with how rmp-serde was exporting its errors,
// and avoids unnecessary code changes.
pub mod decode {
    #[derive(Debug)]
    pub struct Error(serde_cbor::Error);

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "Cbor Decode Error: {}", self.0)
        }
    }

    impl From<serde_cbor::Error> for Error {
        fn from(src: serde_cbor::Error) -> Self {
            Self(src)
        }
    }
}

pub mod encode {
    #[derive(Debug)]
    pub struct Error(serde_cbor::Error);

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "Cbor Encode Error: {}", self.0)
        }
    }

    impl From<serde_cbor::Error> for Error {
        fn from(src: serde_cbor::Error) -> Self {
            Self(src)
        }
    }
}

/// Serialize the given data structure.
///
/// Forward mc_util_serial::serialize to bincode::serialize(..., Infinite)
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// fail.
pub fn serialize<T: ?Sized>(value: &T) -> Result<Vec<u8>, encode::Error>
where
    T: serde::ser::Serialize + Sized,
{
    Ok(serde_cbor::to_vec(value)?)
}

// Forward mc_util_serial::deserialize to bincode::deserialize
pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, decode::Error>
where
    T: serde::de::Deserialize<'a>,
{
    Ok(serde_cbor::from_slice(bytes)?)
}

pub fn encode<T: Message>(value: &T) -> Vec<u8> {
    value.encode_to_vec()
}

pub fn decode<T: Message + Default>(buf: &[u8]) -> Result<T, DecodeError> {
    T::decode(buf)
}

/// Take a prost type and try to roundtrip it through a protobuf type
#[cfg(feature = "test_utils")]
pub fn round_trip_message<SRC: Message + Eq + Default, DEST: protobuf::Message>(prost_val: &SRC) {
    let prost_bytes = encode(prost_val);

    let dest_val =
        DEST::parse_from_bytes(&prost_bytes).expect("Parsing protobuf from prost bytes failed");

    let protobuf_bytes = dest_val
        .write_to_bytes()
        .expect("Writing protobuf to bytes failed");

    let final_val: SRC = decode(&protobuf_bytes).expect("Parsing prost from protobuf bytes failed");

    assert_eq!(
        *prost_val, final_val,
        "Round-trip check failed!\nprost: {:?}\nprotobuf: {:?}",
        prost_val, final_val
    );
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_serialize_string() {
        let the_string = "There goes the baker with his tray, like always";
        let serialized = serialize(&the_string).unwrap();
        let deserialized: &str = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, the_string);
    }

    #[derive(PartialEq, Serialize, Deserialize, Debug)]
    struct TestStruct {
        vec: Vec<u8>,
        integer: u64,
        float: f64,
    }

    #[test]
    fn test_serialize_struct() {
        let the_struct = TestStruct {
            vec: vec![233, 123, 0, 12],
            integer: 4_242_424_242,
            float: 1.2345,
        };
        let serialized = serialize(&the_struct).unwrap();
        let deserialized: TestStruct = deserialize(&serialized).unwrap();
        assert_eq!(deserialized, the_struct);
    }
}
