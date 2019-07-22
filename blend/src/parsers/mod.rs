pub mod blend;
pub mod dna;
pub mod field;
pub mod primitive;

use nom::{
    error::{ErrorKind, ParseError},
    number::Endianness as NomEndianness,
    IResult,
};
use std::io;

type Result<'a, T> = IResult<&'a [u8], T, BlendParseError>;

#[derive(Debug, Copy, Clone)]
pub enum PointerSize {
    Bits32,
    Bits64,
}

impl PointerSize {
    /// Returns the pointer size in bytes
    pub fn bytes_num(self) -> usize {
        match self {
            PointerSize::Bits32 => 4,
            PointerSize::Bits64 => 8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Endianness {
    Little,
    Big,
}

impl From<NomEndianness> for Endianness {
    fn from(e: NomEndianness) -> Endianness {
        match e {
            NomEndianness::Little => Endianness::Little,
            NomEndianness::Big => Endianness::Big,
        }
    }
}

#[derive(Debug)]
pub enum BlendParseError {
    NomError {
        kind: ErrorKind,
        other: Option<Box<BlendParseError>>,
    },
    IoError(io::Error),
    NotEnoughData,
    UnknownBlockCode,
    UnsupportedCountOnPrincipalBlock, // Assumption: principal blocks are always single blocks
    InvalidMemoryAddress,
    NoDnaBlockFound,
}

impl ParseError<&[u8]> for BlendParseError {
    fn from_error_kind(_input: &[u8], kind: ErrorKind) -> Self {
        BlendParseError::NomError { kind, other: None }
    }

    fn append(_input: &[u8], kind: ErrorKind, other: Self) -> Self {
        BlendParseError::NomError {
            kind,
            other: Some(Box::new(other)),
        }
    }
}
