use self::{interrupt::*, interrupt_data::*, resume_data::*};
use super::*;
use crate::consensus::ValidationError;
use derive_more::From;
use enum_as_inner::EnumAsInner;
use ethereum_types::Address;
use ethnum::U256;
use std::{
    ops::{Generator, GeneratorState},
    pin::Pin,
};

/// Interrupts.
pub mod interrupt;
/// Data attached to interrupts.
pub mod interrupt_data;
/// Data required for resume.
pub mod resume_data;

pub(crate) type InnerCoroutine = Box<
    dyn Generator<ResumeData, Yield = InterruptData, Return = Result<(), Box<ValidationError>>>
        + Send
        + Sync
        + Unpin,
>;

#[macro_export]
macro_rules! gen_await {
    ($e:expr) => {{
        let mut resume_data = ResumeData::Empty;
        loop {
            match ::core::pin::Pin::new(&mut $e).resume(resume_data) {
                ::core::ops::GeneratorState::Yielded(interrupt) => {
                    resume_data = yield interrupt;
                }
                ::core::ops::GeneratorState::Complete(result) => break result,
            }
        }
    }};
}

fn resume_interrupt(mut inner: InnerCoroutine, resume_data: ResumeData) -> Interrupt {
    match Pin::new(&mut *inner).resume(resume_data) {
        GeneratorState::Yielded(interrupt) => match interrupt {
            InterruptData::ReadAccount { address } => Interrupt::ReadAccount {
                interrupt: ReadAccountInterrupt { inner },
                address,
            },
            InterruptData::ReadStorage { address, location } => Interrupt::ReadStorage {
                interrupt: ReadStorageInterrupt { inner },
                address,
                location,
            },
            InterruptData::ReadCode { code_hash } => Interrupt::ReadCode {
                interrupt: ReadCodeInterrupt { inner },
                code_hash,
            },
            InterruptData::EraseStorage { address, location } => Interrupt::EraseStorage {
                interrupt: EraseStorageInterrupt { inner },
                address,
                location,
            },
            InterruptData::ReadHeader {
                block_number,
                block_hash,
            } => Interrupt::ReadHeader {
                interrupt: ReadHeaderInterrupt { inner },
                block_number,
                block_hash,
            },
            InterruptData::ReadBody {
                block_number,
                block_hash,
            } => Interrupt::ReadBody {
                interrupt: ReadBodyInterrupt { inner },
                block_number,
                block_hash,
            },
            InterruptData::ReadTotalDifficulty {
                block_number,
                block_hash,
            } => Interrupt::ReadTotalDifficulty {
                interrupt: ReadTotalDifficultyInterrupt { inner },
                block_number,
                block_hash,
            },
            InterruptData::BeginBlock { block_number } => Interrupt::BeginBlock {
                interrupt: BeginBlockInterrupt { inner },
                block_number,
            },
            InterruptData::UpdateAccount {
                address,
                initial,
                current,
            } => Interrupt::UpdateAccount {
                interrupt: UpdateAccountInterrupt { inner },
                address,
                initial,
                current,
            },
            InterruptData::UpdateCode { code_hash, code } => Interrupt::UpdateCode {
                interrupt: UpdateCodeInterrupt { inner },
                code_hash,
                code,
            },
            InterruptData::UpdateStorage {
                address,
                location,
                initial,
                current,
            } => Interrupt::UpdateStorage {
                interrupt: UpdateStorageInterrupt { inner },
                address,
                location,
                initial,
                current,
            },
        },
        GeneratorState::Complete(result) => Interrupt::Complete {
            interrupt: FinishedInterrupt(inner),
            result,
        },
    }
}
