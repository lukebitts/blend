use super::Endianness;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

pub(crate) trait BlendPrimitive {
    fn parse(data: &[u8], endianness: Endianness) -> Self;
    fn blender_name() -> &'static str;
}

impl BlendPrimitive for char {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_u8(data, endianness) as char
    }
    fn blender_name() -> &'static str {
        "char"
    }
}

impl BlendPrimitive for i8 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_i8(data, endianness)
    }
    fn blender_name() -> &'static str {
        "char"
    }
}

impl BlendPrimitive for u8 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_u8(data, endianness)
    }
    fn blender_name() -> &'static str {
        "char"
    }
}

impl BlendPrimitive for u16 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_u16(data, endianness)
    }
    fn blender_name() -> &'static str {
        "ushort"
    }
}

impl BlendPrimitive for i16 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_i16(data, endianness)
    }
    fn blender_name() -> &'static str {
        "short"
    }
}

impl BlendPrimitive for i32 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_i32(data, endianness)
    }
    fn blender_name() -> &'static str {
        "int"
    }
}

impl BlendPrimitive for f32 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_f32(data, endianness)
    }
    fn blender_name() -> &'static str {
        "float"
    }
}

impl BlendPrimitive for f64 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_f64(data, endianness)
    }
    fn blender_name() -> &'static str {
        "double"
    }
}

impl BlendPrimitive for u64 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_u64(data, endianness)
    }
    fn blender_name() -> &'static str {
        "uint64_t"
    }
}

impl BlendPrimitive for i64 {
    fn parse(data: &[u8], endianness: Endianness) -> Self {
        parse_i64(data, endianness)
    }
    fn blender_name() -> &'static str {
        "int64_t"
    }
}

pub fn parse_i8(slice: &[u8], _endianness: Endianness) -> i8 {
    let mut rdr = Cursor::new(slice);
    rdr.read_i8().unwrap()
}

pub fn parse_u8(slice: &[u8], _endianness: Endianness) -> u8 {
    let mut rdr = Cursor::new(slice);
    rdr.read_u8().unwrap()
}

pub fn parse_u16(slice: &[u8], endianness: Endianness) -> u16 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_u16::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_u16::<BigEndian>().unwrap(),
    }
}

pub fn parse_i16(slice: &[u8], endianness: Endianness) -> i16 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_i16::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_i16::<BigEndian>().unwrap(),
    }
}

pub fn parse_i32(slice: &[u8], endianness: Endianness) -> i32 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_i32::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_i32::<BigEndian>().unwrap(),
    }
}

pub fn parse_f32(slice: &[u8], endianness: Endianness) -> f32 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_f32::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_f32::<BigEndian>().unwrap(),
    }
}

pub fn parse_f64(slice: &[u8], endianness: Endianness) -> f64 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_f64::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_f64::<BigEndian>().unwrap(),
    }
}

pub fn parse_u32(slice: &[u8], endianness: Endianness) -> u32 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_u32::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_u32::<BigEndian>().unwrap(),
    }
}

pub fn parse_i64(slice: &[u8], endianness: Endianness) -> i64 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_i64::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_i64::<BigEndian>().unwrap(),
    }
}

pub fn parse_u64(slice: &[u8], endianness: Endianness) -> u64 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_u64::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_u64::<BigEndian>().unwrap(),
    }
}
