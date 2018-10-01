use super::super::{Endianness, PointerSize};
use super::sdna;
use byteorder::{self, ReadBytesExt};
use std::io::{self, Read};

pub fn read_exact<R: io::Read>(r: &mut R, count: usize) -> Result<Vec<u8>, io::Error> {
    let mut v = vec![0; count];
    r.read_exact(&mut v)?;
    Ok(v)
}

fn validate_bytes<R: io::Read>(r: &mut R, validator: &[u8]) -> io::Result<bool> {
    Ok(read_exact(r, validator.len())? == validator)
}

fn skip_to_alignment<R: Read>(reader: &mut R, mut bytes_read: usize) -> Result<usize, io::Error> {
    loop {
        if bytes_read % 4 != 0 {
            reader.read_u8()?;
            bytes_read += 1;
        } else {
            break Ok(bytes_read);
        }
    }
}

fn read_u32<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<u32> {
    match e {
        Endianness::LittleEndian => r.read_u32::<byteorder::LittleEndian>(),
        Endianness::BigEndian => r.read_u32::<byteorder::BigEndian>(),
    }
}

pub fn read_f32<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<f32> {
    match e {
        Endianness::LittleEndian => r.read_f32::<byteorder::LittleEndian>(),
        Endianness::BigEndian => r.read_f32::<byteorder::BigEndian>(),
    }
}

pub fn read_i32<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<i32> {
    match e {
        Endianness::LittleEndian => r.read_i32::<byteorder::LittleEndian>(),
        Endianness::BigEndian => r.read_i32::<byteorder::BigEndian>(),
    }
}

pub fn read_i16<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<i16> {
    match e {
        Endianness::LittleEndian => r.read_i16::<byteorder::LittleEndian>(),
        Endianness::BigEndian => r.read_i16::<byteorder::BigEndian>(),
    }
}

pub fn read_ptr<R: io::Read>(r: &mut R, e: Endianness, s: PointerSize) -> io::Result<u64> {
    Ok(match (e, s) {
        (Endianness::LittleEndian, PointerSize::Bits32) => {
            r.read_u32::<byteorder::LittleEndian>()? as u64
        }
        (Endianness::LittleEndian, PointerSize::Bits64) => {
            r.read_u64::<byteorder::LittleEndian>()?
        }
        (Endianness::BigEndian, PointerSize::Bits32) => {
            r.read_u32::<byteorder::BigEndian>()? as u64
        }
        (Endianness::BigEndian, PointerSize::Bits64) => r.read_u64::<byteorder::BigEndian>()?,
    })
}

fn read_u16<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<u16> {
    match e {
        Endianness::LittleEndian => r.read_u16::<byteorder::LittleEndian>(),
        Endianness::BigEndian => r.read_u16::<byteorder::BigEndian>(),
    }
}

fn read_until<R: io::Read>(r: &mut R, delim: u8) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    loop {
        let c = r.read_u8()?;
        if c == delim {
            break;
        }
        buf.push(c);
    }
    Ok(buf)
}

fn read_string<R: io::Read>(r: &mut R) -> io::Result<String> {
    let variable_name_buf = read_until(r, b'\0')?;

    Ok(String::from_utf8(variable_name_buf).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid variable name (expected utf8 got err: {}", e),
        )
    })?)
}

pub fn parse_blend_header<R: io::Read>(r: &mut R) -> io::Result<sdna::Header> {
    if !validate_bytes(r, b"BLENDER")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid file header (expected BLENDER)",
        ));
    }

    let pointer_size = match r.read_u8()? {
        b'_' => PointerSize::Bits32,
        b'-' => PointerSize::Bits64,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid pointer size (expected _ or -)",
            ))
        }
    };

    let endianness = match r.read_u8()? {
        b'v' => Endianness::LittleEndian,
        b'V' => Endianness::BigEndian,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid endianness (expected v or V)",
            ))
        }
    };

    let version = {
        let v = read_exact(r, 3)?;
        [v[0], v[1], v[2]]
    };

    Ok(sdna::Header {
        pointer_size,
        endianness,
        version,
    })
}

pub fn parse_blend_block_header<R: io::Read>(
    r: &mut R,
    e: Endianness,
    s: PointerSize,
) -> io::Result<sdna::BlockHeader> {
    let code = {
        let v = read_exact(r, 4)?;
        [v[0], v[1], v[2], v[3]]
    };
    let size = read_u32(r, e)?;
    let old_memory_address = read_ptr(r, e, s)?;
    let sdna_index = read_u32(r, e)? as usize;
    let count = read_u32(r, e)?;

    Ok(sdna::BlockHeader {
        code,
        size,
        old_memory_address,
        sdna_index,
        count,
    })
}

pub fn parse_blend_block<R: io::Read>(
    r: &mut R,
    e: Endianness,
    s: PointerSize,
) -> io::Result<sdna::Block> {
    let header = parse_blend_block_header(r, e, s)?;
    let data = read_exact(r, header.size as usize)?;

    Ok(sdna::Block { header, data })
}

pub fn parse_blend_dna<R: io::Read>(r: &mut R, e: Endianness) -> io::Result<sdna::Dna> {
    if !validate_bytes(r, b"SDNANAME")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid dna identifier (expected SDNANAME)",
        ));
    }

    let (bytes_read, names) = {
        let mut names = Vec::new();
        let mut bytes_read = 0;
        let name_count = read_u32(r, e)?;
        for _ in 0..name_count {
            let name = read_string(r)?;
            bytes_read += name.len() + 1;
            names.push(name);
        }
        (bytes_read, names)
    };
    skip_to_alignment(r, bytes_read)?;

    if !validate_bytes(r, b"TYPE")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid dna identifier (expected TYPE)",
        ));
    }

    let mut types: Vec<sdna::Type> = {
        let (bytes_read, types) = {
            let mut types = Vec::new();
            let mut bytes_read = 0;
            let type_count = read_u32(r, e)?;
            for _ in 0..type_count {
                let type_ = read_string(r)?;
                bytes_read += type_.len() + 1;
                types.push(type_);
            }
            (bytes_read, types)
        };
        skip_to_alignment(r, bytes_read)?;

        if !validate_bytes(r, b"TLEN")? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid dna identifier (expected TLEN)",
            ));
        }

        let (bytes_read, type_lengths) = {
            let mut type_lengths = Vec::new();
            let mut bytes_read = 0;
            for _ in 0..types.len() {
                type_lengths.push(read_u16(r, e)?);
                bytes_read += 2;
            }
            (bytes_read, type_lengths)
        };
        skip_to_alignment(r, bytes_read)?;

        type_lengths
            .iter()
            .zip(types)
            .map(|(&s, t)| sdna::Type {
                name: t,
                length: s,
                is_primitive: true,
            }).collect()
    };

    if !validate_bytes(r, b"STRC")? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid dna identifier (expected STRC)",
        ));
    }

    let structures = {
        let mut structures = Vec::new();
        let structure_count = read_u32(r, e)?;
        for _ in 0..structure_count {
            let type_index = read_u16(r, e)? as usize;
            let field_count = read_u16(r, e)?;
            let mut fields = Vec::new();
            for _ in 0..field_count {
                let type_index = read_u16(r, e)? as usize;
                let name_index = read_u16(r, e)? as usize;
                fields.push(sdna::Field {
                    type_index,
                    name_index,
                });
            }

            types[type_index as usize].is_primitive = false;

            structures.push(sdna::StructureTemplate { type_index, fields });
        }
        structures
    };

    Ok(sdna::Dna {
        names,
        types,
        structures,
    })
}

pub fn parse_blend_file<R: io::Read>(r: &mut R) -> io::Result<sdna::Blend> {
    let header = parse_blend_header(r)?;

    let (blocks, dna) = {
        let mut file_blocks = Vec::new();
        let mut dna = None;
        loop {
            let block = parse_blend_block(r, header.endianness, header.pointer_size)?;

            if &block.header.code == b"ENDB" {
                break;
            } else if &block.header.code == b"DNA1" {
                dna = Some(parse_blend_dna(
                    &mut io::Cursor::new(block.data),
                    header.endianness,
                )?);
            } else {
                file_blocks.push(block)
            }
        }
        (
            file_blocks,
            dna.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Expected dna block"))?,
        )
    };

    Ok(sdna::Blend {
        header,
        blocks,
        dna,
    })
}
