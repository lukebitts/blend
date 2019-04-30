use blend_parse::Endianness;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

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
