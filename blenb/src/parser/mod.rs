//! # Example
//!
//! ```rust
//! use blend_parse::Blend;
//!
//! fn main() {
//!     let blend = Blend::from_path("your_blend.blend").unwrap();
//!
//!     for block in blend.blocks {
//!         match &block.header.code {
//!             b"GLOB" => println!("GLOB"),
//!             b"DATA" => println!("DATA"),
//!             n => (),
//!         }
//!     }
//! }
//! ```

pub mod blend_parser;
pub mod primitive_parsers;
pub mod struct_parser;
pub mod field_parser;

use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read};
use std::path::Path;
use std::num::NonZeroU64;

/// Pointer fields inside the .blend file can have either 32 or 64 bits
/// depending on the computer used to save the file.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

/// Data can be little endian or big endian depending on the computer
/// used to save the file.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Endianness {
    LittleEndian,
    BigEndian,
}

impl Into<nom::Endianness> for Endianness {
    fn into(self) -> nom::Endianness {
        match self {
            Endianness::LittleEndian => nom::Endianness::Little,
            Endianness::BigEndian => nom::Endianness::Big,
        }
    }
}

/// Header of the .blend file.
#[derive(Debug, Clone)]
pub struct Header {
    /// Size of the pointers inside the file.
    pub pointer_size: PointerSize,
    /// Endianness of the values.
    pub endianness: Endianness,
    /// Blender version, example: `[b'2', b'8', b'0']`.
    pub version: [u8; 3],
}

/// Header of a file-block.
#[derive(Debug)]
pub struct BlockHeader {
    /// The file-block code. A material block would have a code of `[b'M', b'A', 0, 0]`, a camera block would have `[b'C', b'A', 0, 0]`, etc.
    pub code: [u8; 4],
    /// The size in bytes of this block's data. Should be the same number as the `Block::data::len`.
    pub size: u32,
    /// Blender dumps its memory into the .blend file and some blocks have pointers to other blocks, this address is used to follow these pointers.
    pub old_memory_address: NonZeroU64,
    /// In some cases represents the type of the file-block in the DNA array of types, see [blend_sdna](todo:add_link) for more.
    pub sdna_index: u32,
    /// A file-block can contain more than one struct.
    pub count: u32,
}

/// The file-block
pub struct Block {
    pub header: BlockHeader,
    pub data: Vec<u8>,
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Block")
            .field("header", &self.header)
            .field("data_len", &self.data.len())
            .finish()
    }
}

/// Returned whenever the .blend file can't be parsed.
#[derive(Debug)]
pub enum BlendError {
    Io(io::Error),
    InvalidData,
}

/// The loaded .blend file contains the file's header information and a list of blocks.
#[derive(Debug)]
pub struct Blend {
    pub header: Header,
    pub blocks: Vec<Block>,
}

impl Blend {
    /// Returns a new `Blend` instance from `data`.
    pub fn from_data<T: Read>(mut data: T) -> Result<Self, BlendError> {
        let mut buffer = Vec::new();
        data.read_to_end(&mut buffer).map_err(BlendError::Io)?;

        let parser = blend_parser::BlendParseContext::default();

        let res = parser.blend(&buffer);

        match res {
            (_, Ok((_, blend))) => Ok(blend),
            (_, Err(_)) => Err(BlendError::InvalidData),
        }
    }

    /// Returns a new `Blend` instance from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, BlendError> {
        use std::fs::File;

        let file = File::open(path).map_err(BlendError::Io)?;
        Blend::from_data(file)
    }
}
