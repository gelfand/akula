use crate::{models::*, util::*};
use arrayvec::ArrayVec;
use bytes::Bytes;
use educe::Educe;
use ethereum_types::*;
use modular_bitfield::prelude::*;
use rlp_derive::*;
use serde::*;
use static_bytes::Buf;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub nonce: u64,
    pub balance: U256,
    pub code_hash: H256, // hash of the bytecode
    pub incarnation: Incarnation,
}

#[derive(Debug, RlpEncodable, RlpDecodable)]
pub struct RlpAccount {
    pub nonce: u64,
    pub balance: U256,
    pub storage_root: H256,
    pub code_hash: H256,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            nonce: 0,
            balance: U256::zero(),
            code_hash: EMPTY_HASH,
            incarnation: 0,
        }
    }
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct SerializedAccount {
    pub balance: U256,
    #[serde(deserialize_with = "deserialize_str_as_bytes")]
    #[educe(Debug(method = "write_hex_string"))]
    pub code: Bytes<'static>,
    pub nonce: U64,
    pub storage: HashMap<U256, U256>,
}
#[allow(dead_code)]
#[bitfield]
#[derive(Clone, Copy, Debug, Default)]
struct AccountStorageFlags {
    nonce: bool,
    balance: bool,
    incarnation: bool,
    code_hash: bool,
    dummy: B4,
}

pub type EncodedAccount = ArrayVec<u8, { Account::MAX_ENCODED_LEN }>;

impl Account {
    pub const MAX_ENCODED_LEN: usize = 1 + (1 + 32) + (1 + 8) + (1 + 32) + (1 + 8);

    pub fn encoding_length_for_storage(&self) -> usize {
        let mut struct_length = 1; // 1 byte for fieldset

        if !self.balance.is_zero() {
            struct_length += 1 + Self::u256_compact_len(self.balance);
        }

        if self.nonce > 0 {
            struct_length += 1 + Self::u64_compact_len(self.nonce);
        }

        if self.code_hash != EMPTY_HASH {
            struct_length += 33 // 32-byte array + 1 byte for length
        }

        if self.incarnation > 0 {
            struct_length += 1 + Self::u64_compact_len(self.incarnation);
        }

        struct_length
    }

    pub fn encode_for_storage(self, omit_code_hash: bool) -> EncodedAccount {
        fn u256_compact_len(num: U256) -> usize {
            (num.bits() + 7) / 8
        }
        fn u64_compact_len(num: u64) -> usize {
            ((u64::BITS - num.leading_zeros()) as usize + 7) / 8
        }
        fn write_compact(input: &[u8], buffer: &mut [u8]) -> usize {
            let mut written = 0;
            for &byte in input.iter().skip_while(|v| **v == 0) {
                written += 1;
                buffer[written] = byte;
            }
            if written > 0 {
                buffer[0] = written as u8;
            }

            written
        }

        let mut buffer = vec![0; self.encoding_length_for_storage()];

        let mut field_set = AccountStorageFlags::default(); // start with first bit set to 0
        let mut pos = 1;
        if self.nonce > 0 {
            field_set.set_nonce(true);
            pos += 1 + Self::write_compact(&self.nonce.to_be_bytes(), &mut buffer[pos..]);
        }

        // Encoding balance
        if !self.balance.is_zero() {
            field_set.set_balance(true);
            pos += 1 + Self::write_compact(&value_to_bytes(self.balance), &mut buffer[pos..]);
        }

        if self.incarnation > 0 {
            field_set.set_incarnation(true);
            pos += 1 + Self::write_compact(&self.incarnation.to_be_bytes(), &mut buffer[pos..]);
        }

        // Encoding code hash
        if self.code_hash != EMPTY_HASH && !self.omit_code_hash.unwrap_or(false) {
            field_set.set_code_hash(true);
            buffer[pos] = 32;
            buffer[pos + 1..pos + 33].copy_from_slice(self.code_hash.as_bytes());
        }

        let fs = field_set.into_bytes()[0];
        buffer[0] = fs;

        buffer.into()
    }

    pub fn decode_from_storage(enc: &[u8]) -> Self {
        fn bytes_to_u64(buf: &[u8]) -> u64 {
            let mut decoded = [0u8; 8];
            for (i, b) in buf.iter().rev().enumerate() {
                decoded[i] = *b;
            }

            u64::from_le_bytes(decoded)
        }

        let mut a = Self::default();

        let field_set_flag = enc.get_u8();
        let field_set = AccountStorageFlags::from_bytes(field_set_flag.to_be_bytes());

        if field_set.nonce() {
            let decode_length = enc.get_u8() as usize;

            a.nonce = bytes_to_u64(&enc[..decode_length]);
            enc.advance(decode_length);
        }

        if field_set.balance() {
            let decode_length = enc.get_u8() as usize;

            a.balance = U256::from_big_endian(&enc[..decode_length]);
            enc.advance(decode_length);
        }

        if field_set.incarnation() {
            let decode_length = enc.get_u8() as usize;

            a.incarnation = bytes_to_u64(&enc[..decode_length]);
            enc.advance(decode_length);
        }

        if field_set.code_hash() {
            let decode_length = enc.get_u8() as usize;

            // if decode_length != 32 {
            //     return Err(InvalidLength { got: decode_length });
            // }

            a.code_hash = H256::from_slice(&enc[..decode_length]);
            enc.advance(decode_length);
        }

        Ok(Some(a))
    }

    pub fn to_rlp(&self, storage_root: H256) -> RlpAccount {
        RlpAccount {
            nonce: self.nonce,
            balance: self.balance,
            storage_root,
            code_hash: self.code_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::*;
    use hex_literal::hex;

    fn run_test_storage(original: Account, expected_encoded: &[u8]) {
        let encoded_account = original.encode_for_storage(false);

        assert_eq!(encoded_account, expected_encoded);

        let decoded = Account::decode_for_storage(&encoded_account)
            .unwrap()
            .unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn empty() {
        run_test_storage(
            Account {
                nonce: 100,
                balance: U256::zero(),
                code_hash: EMPTY_HASH,
                incarnation: 5,
            },
            &hex!("0501640105"),
        )
    }

    #[test]
    fn with_code() {
        run_test_storage(
            Account {
                nonce: 2,
                balance: 1000.into(),
                code_hash: keccak256(&[1, 2, 3]),
                incarnation: 4,
            },
            &hex!("0f01020203e8010420f1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239"),
        )
    }

    #[test]
    fn with_code_with_storage_size_hack() {
        run_test_storage(Account {
            nonce: 2,
            balance: 1000.into(),
            code_hash: keccak256(&[1, 2, 3]),
            incarnation: 5,
        }, &hex!("0f01020203e8010520f1885eda54b7a053318cd41e2093220dab15d65381b1157a3633a83bfd5c9239"))
    }

    #[test]
    fn without_code() {
        run_test_storage(
            Account {
                nonce: 2,
                balance: 1000.into(),
                code_hash: EMPTY_HASH,
                incarnation: 5,
            },
            &hex!("0701020203e80105"),
        )
    }

    #[test]
    fn with_empty_balance_non_nil_contract_and_not_zero_incarnation() {
        run_test_storage(
            Account {
                nonce: 0,
                balance: 0.into(),
                code_hash: H256(hex!(
                    "0000000000000000000000000000000000000000000000000000000000000123"
                )),
                incarnation: 1,
            },
            &hex!("0c0101200000000000000000000000000000000000000000000000000000000000000123"),
        )
    }

    #[test]
    fn with_empty_balance_and_not_zero_incarnation() {
        run_test_storage(
            Account {
                nonce: 0,
                balance: 0.into(),
                code_hash: EMPTY_HASH,
                incarnation: 1,
            },
            &hex!("040101"),
        )
    }
}
