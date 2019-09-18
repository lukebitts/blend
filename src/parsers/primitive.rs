use super::Endianness;
use nom::number::complete::{
    be_f32, be_f64, be_i16, be_i32, be_i64, be_i8, be_u16, be_u32, be_u64, le_f32, le_f64, le_i16,
    le_i32, le_i64, le_i8, le_u16, le_u32, le_u64,
};

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

pub fn parse_i8(slice: &[u8], endianness: Endianness) -> i8 {
    let (_, val) = match endianness {
        Endianness::Little => le_i8::<()>(slice).expect("parse i8"),
        Endianness::Big => be_i8::<()>(slice).expect("parse i8"),
    };
    val
}

pub fn parse_u8(slice: &[u8], _endianness: Endianness) -> u8 {
    *slice.get(0).expect("parse u8")
}

pub fn parse_u16(slice: &[u8], endianness: Endianness) -> u16 {
    let (_, val) = match endianness {
        Endianness::Little => le_u16::<()>(slice).expect("parse u16"),
        Endianness::Big => be_u16::<()>(slice).expect("parse u16"),
    };
    val
}

pub fn parse_i16(slice: &[u8], endianness: Endianness) -> i16 {
    let (_, val) = match endianness {
        Endianness::Little => le_i16::<()>(slice).expect("parse i16"),
        Endianness::Big => be_i16::<()>(slice).expect("parse i16"),
    };
    val
}

pub fn parse_i32(slice: &[u8], endianness: Endianness) -> i32 {
    let (_, val) = match endianness {
        Endianness::Little => le_i32::<()>(slice).expect("parse i32"),
        Endianness::Big => be_i32::<()>(slice).expect("parse i32"),
    };
    val
}

pub fn parse_f32(slice: &[u8], endianness: Endianness) -> f32 {
    let (_, val) = match endianness {
        Endianness::Little => le_f32::<()>(slice).expect("parse f32"),
        Endianness::Big => be_f32::<()>(slice).expect("parse f32"),
    };
    val
}

pub fn parse_f64(slice: &[u8], endianness: Endianness) -> f64 {
    let (_, val) = match endianness {
        Endianness::Little => le_f64::<()>(slice).expect("parse f64"),
        Endianness::Big => be_f64::<()>(slice).expect("parse f64"),
    };
    val
}

pub fn parse_u32(slice: &[u8], endianness: Endianness) -> u32 {
    let (_, val) = match endianness {
        Endianness::Little => le_u32::<()>(slice).expect("parse u32"),
        Endianness::Big => be_u32::<()>(slice).expect("parse u32"),
    };
    val
}

pub fn parse_i64(slice: &[u8], endianness: Endianness) -> i64 {
    let (_, val) = match endianness {
        Endianness::Little => le_i64::<()>(slice).expect("parse i64"),
        Endianness::Big => be_i64::<()>(slice).expect("parse i64"),
    };
    val
}

pub fn parse_u64(slice: &[u8], endianness: Endianness) -> u64 {
    let (_, val) = match endianness {
        Endianness::Little => le_u64::<()>(slice).expect("parse u64"),
        Endianness::Big => be_u64::<()>(slice).expect("parse u64"),
    };
    val
}
