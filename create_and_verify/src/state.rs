//! State transition types

use {
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        entrypoint::ProgramResult,program_error::ProgramError,
        program_pack::IsInitialized, pubkey::Pubkey
    },
};
use std::mem::size_of;

/// Track data
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct TrackData {
    /// user ID
    pub user_id: String,
    /// track ID
    pub track_id: String,
    /// track source
    pub source: String,
}

/*
/// SigInfo
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub struct VerifierInfo {
    /// Solana pub key for verifier
    pub verifier: Pubkey,
    /*
    /// Signer nonce
    /// Used to indicate whether this account is initialized or not
    pub version: u8,
    /// Nonce
    pub nonce: u64
    */
}
*/

/// SigInfo
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VerifierInfo {
    /// Pubkey of the account authorized to add/remove valid signers
    pub owner: Pubkey,
    /// Groups version
    pub nonce: u64,
}

impl VerifierInfo {
    /// Length of VerifierInfo when serialized
    pub const LEN: usize = size_of::<VerifierInfo>();

    /// Deserialize a byte buffer into VerifierInfo
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        #[allow(clippy::cast_ptr_alignment)]
        let signer_group: &VerifierInfo =
            unsafe { &*(&input[0] as *const u8 as *const VerifierInfo) };
        Ok(*signer_group)
    }

    /// Serialize a VerifierInfo struct into byte buffer
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe { &mut *(&mut output[0] as *mut u8 as *mut VerifierInfo) };
        *value = *self;
        Ok(())
    }

    /// Check if VerifierInfo is initialized
    pub fn is_initialized(&self) -> bool {
        self.nonce != 0
    }
}