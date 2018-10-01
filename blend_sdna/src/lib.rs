extern crate byteorder;
extern crate lazy_static;
extern crate regex;

pub mod sdna;

#[derive(Debug, Copy, Clone)]
pub enum Endianness {
    LittleEndian,
    BigEndian,
}

#[derive(Debug, Copy, Clone)]
pub enum PointerSize {
    Bits32,
    Bits64,
}

impl PointerSize {
    pub fn length_in_bytes(&self) -> u8 {
        match *self {
            PointerSize::Bits32 => 4,
            PointerSize::Bits64 => 8,
        }
    }
}
