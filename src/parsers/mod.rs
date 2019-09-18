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

/// Size of a pointer on the machine used to create the .blend file.
#[derive(Debug, Copy, Clone)]
pub enum PointerSize {
    Bits32,
    Bits64,
}

impl PointerSize {
    /// Returns the pointer size in bytes.
    pub fn bytes_num(self) -> usize {
        match self {
            PointerSize::Bits32 => 4,
            PointerSize::Bits64 => 8,
        }
    }
}

/// Endianness of the machine used to create the .blend file.
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

/// Errors that can happen during the initial parsing of the .blend file.
/// Most errors are simply `NomError` but a few of them are specific either
/// for better error reporting or due to custom logic.
#[derive(Debug)]
pub enum BlendParseError {
    NomError {
        kind: ErrorKind,
        other: Option<Box<BlendParseError>>,
    },
    IoError(io::Error),
    /// Returned when the file is incomplete.
    NotEnoughData,
    /// The known block codes are `b"REND"`, `b"TEST"`, `b"GLOB"`, `b"DATA"` and any two-digit code
    /// like `b"OB\0\0" for objects. Anything different from that returns `UnknownBlockCode`
    UnknownBlockCode,
    /// Principal blocks are assumed to never be lists even though it is possible. This is done
    /// to simplify the API. No version of a blend file was found where this isn't true.
    UnsupportedCountOnPrincipalBlock,
    /// This error happens if a block has a memory address equal to `0`. This should be impossible
    /// as `0` represents a null pointer.
    InvalidMemoryAddress,
    /// Returned when the DNA block is not found at the end of the blend file.
    NoDnaBlockFound,
    /// Returned when the file doesn't start with `b"BLENDER"`. The assumption is that the file
    /// is a gzip compressed blend file, but this isn't actually tested for.
    CompressedFileNotSupported,
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
