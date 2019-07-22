use crate::parsers::{
    dna::{Dna, DnaParseContext},
    BlendParseError, Endianness, PointerSize, Result,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    multi::many_till,
    number::complete::{be_u32, be_u64, le_u32, le_u64},
    sequence::tuple,
    Err,
};
use std::{
    convert::TryInto,
    fmt::{self, Debug, Formatter},
    io::Read,
    num::NonZeroU64,
    path::Path,
    result::Result as StdResult,
};

pub struct BlockData {
    pub data: Vec<u8>,
    pub count: usize,
}

impl Debug for BlockData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "len/count: {}/{}", self.data.len(), self.count)
    }
}

#[derive(Debug)]
pub enum Block {
    Rend,
    Test,
    Global,
    Principal {
        code: [u8; 2],
        memory_address: NonZeroU64,
        dna_index: usize,
        data: BlockData,
    },
    Subsidiary {
        memory_address: NonZeroU64,
        dna_index: usize,
        data: BlockData,
    },
    Dna(Dna),
}

#[derive(Debug, Clone)]
pub struct Header {
    pub pointer_size: PointerSize,
    pub endianness: Endianness,
    pub version: [u8; 3],
}

fn pointer_size_bits32(input: &[u8]) -> Result<PointerSize> {
    let (input, _) = tag("_")(input)?;
    Ok((input, PointerSize::Bits32))
}

fn pointer_size_bits64(input: &[u8]) -> Result<PointerSize> {
    let (input, _) = tag("-")(input)?;
    Ok((input, PointerSize::Bits64))
}

pub fn pointer_size(input: &[u8]) -> Result<PointerSize> {
    alt((pointer_size_bits32, pointer_size_bits64))(input)
}

fn endianness_litte(input: &[u8]) -> Result<Endianness> {
    let (input, _) = tag("v")(input)?;
    Ok((input, Endianness::Little))
}

fn endianness_big(input: &[u8]) -> Result<Endianness> {
    let (input, _) = tag("V")(input)?;
    Ok((input, Endianness::Big))
}

pub fn endianness(input: &[u8]) -> Result<Endianness> {
    alt((endianness_litte, endianness_big))(input)
}

pub fn version(input: &[u8]) -> Result<[u8; 3]> {
    let (input, v) = take(3_usize)(input)?;
    Ok((input, [v[0], v[1], v[2]]))
}

pub fn header(input: &[u8]) -> Result<Header> {
    let (input, _) = tag("BLENDER")(input)?;
    let (input, (pointer_size, endianness, version)) =
        tuple((pointer_size, endianness, version))(input)?;

    Ok((
        input,
        Header {
            pointer_size,
            endianness,
            version,
        },
    ))
}

pub fn block_header_code(input: &[u8]) -> Result<[u8; 4]> {
    let (input, v) = take(4_usize)(input)?;
    Ok((input, [v[0], v[1], v[2], v[3]]))
}

#[derive(Debug)]
pub struct Blend {
    pub header: Header,
    pub blocks: Vec<Block>,
    pub dna: Dna,
}

impl Blend {
    /// Returns a new `Blend` instance from `data`.
    pub fn from_data<T: Read>(mut data: T) -> StdResult<Self, BlendParseError> {
        let mut buffer = Vec::new();
        data.read_to_end(&mut buffer)
            .map_err(BlendParseError::IoError)?;

        let mut parser = BlendParseContext::default();
        let res = parser.blend(&buffer);

        match res {
            Ok((_, blend)) => Ok(blend),
            Err(Err::Failure(e)) | Err(Err::Error(e)) => Err(e),
            Err(Err::Incomplete(..)) => Err(BlendParseError::NotEnoughData),
        }
    }

    /// Returns a new `Blend` instance from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> StdResult<Self, BlendParseError> {
        use std::fs::File;

        let file = File::open(path).map_err(BlendParseError::IoError)?;
        Blend::from_data(file)
    }
}

pub enum BlendParseContext {
    Empty,
    ParsedHeader(Header),
}

impl Default for BlendParseContext {
    fn default() -> Self {
        BlendParseContext::Empty
    }
}

impl BlendParseContext {
    fn memory_address<'a>(&self, input: &'a [u8]) -> Result<'a, NonZeroU64> {
        match self {
            BlendParseContext::ParsedHeader(header) => {
                let read_len: usize = match header.pointer_size {
                    PointerSize::Bits32 => 4,
                    PointerSize::Bits64 => 8,
                };

                let (input, data) = take(read_len)(input)?;

                let (_, address) = match (&header.endianness, &header.pointer_size) {
                    (Endianness::Little, PointerSize::Bits32) => {
                        le_u32(data).map(|(i, n)| (i, u64::from(n)))?
                    }
                    (Endianness::Big, PointerSize::Bits32) => {
                        be_u32(data).map(|(i, n)| (i, u64::from(n)))?
                    }
                    (Endianness::Little, PointerSize::Bits64) => le_u64(data)?,
                    (Endianness::Big, PointerSize::Bits64) => be_u64(data)?,
                };

                if let Some(address) = NonZeroU64::new(address) {
                    Ok((input, address))
                } else {
                    Err(Err::Failure(BlendParseError::InvalidMemoryAddress))
                }
            }
            BlendParseContext::Empty => unreachable!("Header should be parsed here"),
        }
    }

    /// Panics if a u32 can't be converted to usize in your system.
    fn block<'a, 'b>(&'a self, input: &'b [u8]) -> Result<'b, Block>
    where
        'b: 'a,
    {
        match self {
            BlendParseContext::ParsedHeader(header) => {
                let (input, code) = block_header_code(input)?;
                let (input, size): (_, usize) = match header.endianness {
                    Endianness::Little => {
                        le_u32(input).map(|(i, n)| (i, n.try_into().expect("u32 to usize")))?
                    }
                    Endianness::Big => {
                        be_u32(input).map(|(i, n)| (i, n.try_into().expect("u32 to usize")))?
                    }
                };
                let (input, memory_address) = self.memory_address(input)?;
                let (input, dna_index) = match header.endianness {
                    Endianness::Little => le_u32(input)?,
                    Endianness::Big => be_u32(input)?,
                };
                let (input, count) = match header.endianness {
                    Endianness::Little => le_u32(input)?,
                    Endianness::Big => be_u32(input)?,
                };

                let (input, block_data) = take(size)(input)?;

                //Assumption: These block codes will always exist
                let block = match &code {
                    b"REND" => Block::Rend,
                    b"TEST" => Block::Test,
                    b"GLOB" => Block::Global,
                    b"DATA" => Block::Subsidiary {
                        memory_address,
                        dna_index: dna_index.try_into().expect("u32 to usize"),
                        data: BlockData {
                            data: block_data.to_vec(),
                            count: count.try_into().expect("u32 to usize"),
                        },
                    },
                    b"DNA1" => {
                        let ctx = DnaParseContext::new(header.endianness, header.pointer_size);
                        let (_, dna) = ctx.dna(block_data)?;

                        Block::Dna(dna)
                    }
                    &[code1, code2, 0, 0] => {
                        if count != 1 {
                            return Err(Err::Failure(
                                BlendParseError::UnsupportedCountOnPrincipalBlock,
                            ));
                        } else {
                            Block::Principal {
                                code: [code1, code2],
                                memory_address,
                                dna_index: dna_index.try_into().expect("u32 to usize"),
                                data: BlockData {
                                    data: block_data.to_vec(),
                                    count: 1,
                                },
                            }
                        }
                    }
                    _ => return Err(Err::Failure(BlendParseError::UnknownBlockCode)),
                };

                Ok((input, block))
            }
            BlendParseContext::Empty => unreachable!("Header should be parsed here"),
        }
    }

    pub fn blend<'a, 'b>(&'a mut self, input: &'b [u8]) -> Result<'b, Blend>
    where
        'b: 'a,
    {
        let (input, header) = header(input)?;

        //This has to happen before the rest of the parser runs
        *self = BlendParseContext::ParsedHeader(header.clone());

        let (input, (mut blocks, _)) = many_till(move |d| self.block(d), tag("ENDB"))(input)?;

        let dna = if let Some(dna) = blocks.pop() {
            // Assumption: The DNA block is always the last one
            match dna {
                Block::Dna(dna) => dna,
                _ => return Err(Err::Failure(BlendParseError::NoDnaBlockFound)),
            }
        } else {
            return Err(Err::Failure(BlendParseError::NoDnaBlockFound));
        };

        Ok((
            input,
            Blend {
                blocks,
                dna,
                header,
            },
        ))
    }
}
