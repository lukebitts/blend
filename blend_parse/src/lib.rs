#[macro_use]
extern crate nom;

pub mod parser;

use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PointerSize {
    Bits32,
    Bits64,
}

impl PointerSize {
    pub fn bytes_num(&self) -> usize {
        match self {
            PointerSize::Bits32 => 4,
            PointerSize::Bits64 => 8,
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct Header {
    pub pointer_size: PointerSize,
    pub endianness: Endianness,
    pub version: [u8; 3],
}

#[derive(Debug)]
pub struct BlockHeader {
    pub code: [u8; 4],
    pub size: u32,
    pub old_memory_address: u64,
    pub sdna_index: u32,
    pub count: u32,
}

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

#[derive(Debug)]
pub enum BlendParseError {
    Io(io::Error),
    InvalidData,
}

#[derive(Debug)]
pub struct Blend {
    pub header: Header,
    pub blocks: Vec<Block>,
}

impl Blend {
    pub fn new<T: Read>(mut data: T) -> Result<Self, BlendParseError> {
        let mut buffer = Vec::new();
        data.read_to_end(&mut buffer)
            .map_err(|e| BlendParseError::Io(e))?;

        let parser = parser::BlendParseContext::default();

        let res = parser.blend(&buffer);

        match res {
            (_, Ok((_, blend))) => Ok(blend),
            (_, Err(_)) => Err(BlendParseError::InvalidData),
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, BlendParseError> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path).map_err(|e| BlendParseError::Io(e))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| BlendParseError::Io(e))?;

        let parser = parser::BlendParseContext::default();

        let res = parser.blend(&buffer);

        match res {
            (_, Ok((_, blend))) => Ok(blend),
            (_, Err(_)) => Err(BlendParseError::InvalidData),
        }
    }
}
