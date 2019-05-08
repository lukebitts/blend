//! The nom parser used by the library. It is recommended to use `Blend::new` instead of
//! the `parser::BlendParseContext` directly.

use nom::{IResult, Err, Needed, be_u64, be_u32, le_u64, le_u32, named, alt, do_parse, tag, method, u32, call_m, map, many_till, take};
use super::{PointerSize, Endianness, Block, BlockHeader, Header, Blend};
use std::num::NonZeroU64;

named!(pointer_size < &[u8], PointerSize >,
    alt!(
        do_parse!(tag!("_") >> (PointerSize::Bits32)) | 
        do_parse!(tag!("-") >> (PointerSize::Bits64))
    )
);

named!(endianness < &[u8], Endianness >,
    alt!(
        do_parse!(tag!("v") >> (Endianness::LittleEndian)) | 
        do_parse!(tag!("V") >> (Endianness::BigEndian))
    )
);

fn version(i:&[u8]) -> IResult<&[u8], [u8;3]>{
    if i.len() < 3 {
        Err(Err::Incomplete(Needed::Size(3-i.len())))
    } else {
        Ok((&i[3..], [i[0], i[1], i[2]]))  
    }
}

named!(header < &[u8], Header >, 
    do_parse!(
        tag!("BLENDER") >>
        pointer_size: pointer_size >>
        endianness: endianness >>
        version: version >>
        (Header { pointer_size, endianness, version })
    )
);

fn block_header_code(i:&[u8]) -> IResult<&[u8], [u8;4]>{
    if i.len() < 4 {
        Err(Err::Incomplete(Needed::Size(4-i.len())))
    } else {
        Ok((&i[4..], [i[0], i[1], i[2], i[3]]))  
    }
}

#[derive(Debug)]
pub struct BlendParseContext {
    endianness: Endianness,
    pointer_size: PointerSize,
}

impl Default for BlendParseContext {
    //todo: BlendParseContext could be an `enum { Uninit, ParsedHeader(Endianness, PointerSize) }`
    // this way we avoid having the `default` method return a default endianness and pointer size.
    fn default() -> Self {
        Self {
            endianness: Endianness::LittleEndian,
            pointer_size: PointerSize::Bits32,
        }
    }
}

impl BlendParseContext {
    fn old_memory_address(self, i:&[u8]) -> (Self, IResult<&[u8], NonZeroU64>) {
        let read_len = match self.pointer_size {
            PointerSize::Bits32 => 4,
            PointerSize::Bits64 => 8,
        };

        if i.len() < read_len {
            (self, Err(Err::Incomplete(Needed::Size(read_len-i.len()))))
        } else {
            let address = match (self.pointer_size, self.endianness) {
                (PointerSize::Bits32, Endianness::LittleEndian) => 
                    le_u32(i).map(|(u, n)| (u, u64::from(n))),
                (PointerSize::Bits64, Endianness::LittleEndian) => 
                    le_u64(i),
                (PointerSize::Bits32, Endianness::BigEndian) => 
                    be_u32(i).map(|(u, n)| (u, u64::from(n))),
                (PointerSize::Bits64, Endianness::BigEndian) =>  
                    be_u64(i),
            };

            (self, address.and_then(|(rest, address)| {
                match NonZeroU64::new(address) {
                    Some(address) => {
                        Ok((rest, address))
                    }
                    None => {
                        Err(Err::Failure(nom::Context::Code(rest, nom::ErrorKind::Tag)))
                    }
                }
            }))

            //
        }
    }

    method!(block_header < BlendParseContext, &[u8], BlockHeader>, mut self,
        do_parse!(
            code: block_header_code >>
            size: u32!(self.endianness.into()) >>
            old_memory_address: call_m!(self.old_memory_address) >>
            sdna_index: u32!(self.endianness.into()) >>
            count: u32!(self.endianness.into()) >>
            ( BlockHeader { 
                code,  
                size,
                old_memory_address,
                sdna_index,
                count,
            } )
        )
    );

    method!(block < BlendParseContext, &[u8], Block>, mut self,
        do_parse!(
            header: call_m!(self.block_header) >>
            data: take!(header.size) >>
            ( Block { header, data: Vec::from(data) } )
        )
    );

    method!(pub blend < BlendParseContext, &[u8], Blend >, mut self,
        do_parse!(
            header: map!(header, |h| {
                self.endianness = h.endianness;
                self.pointer_size = h.pointer_size;
                h
            }) >>
            blocks: many_till!(
                call_m!(self.block), 
                tag!("ENDB")) >>
            ( Blend { header, blocks: blocks.0 } )
        ) 
    );
}