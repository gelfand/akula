use bytes::Bytes;

use super::*;

#[derive(Debug)]
pub enum InterruptData {
    ReadAccount {
        address: Address,
    },
    ReadStorage {
        address: Address,
        location: U256,
    },
    ReadCode {
        code_hash: H256,
    },
    EraseStorage {
        address: Address,
        location: U256,
    },
    ReadHeader {
        block_number: BlockNumber,
        block_hash: H256,
    },
    ReadBody {
        block_number: BlockNumber,
        block_hash: H256,
    },
    ReadTotalDifficulty {
        block_number: BlockNumber,
        block_hash: H256,
    },
    BeginBlock {
        block_number: BlockNumber,
    },
    UpdateAccount {
        address: Address,
        initial: Option<Account>,
        current: Option<Account>,
    },
    UpdateCode {
        code_hash: H256,
        code: Bytes,
    },
    UpdateStorage {
        address: Address,
        location: U256,
        initial: U256,
        current: U256,
    },
}
