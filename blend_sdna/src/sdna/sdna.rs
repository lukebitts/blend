use super::super::{Endianness, PointerSize};

#[derive(Debug)]
pub struct Field {
    pub type_index: usize,
    pub name_index: usize,
}

#[derive(Debug)]
pub struct StructureTemplate {
    pub type_index: usize,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Type {
    pub name: String,
    pub length: u16,
    pub is_primitive: bool,
}

#[derive(Debug)]
pub struct Dna {
    pub names: Vec<String>,
    pub types: Vec<Type>,
    pub structures: Vec<StructureTemplate>,
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub code: [u8; 4],
    pub size: u32,
    pub old_memory_address: u64,
    pub sdna_index: usize,
    pub count: u32,
}

#[derive(Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub pointer_size: PointerSize,
    pub endianness: Endianness,
    pub version: [u8; 3],
}

#[derive(Debug)]
pub struct Blend {
    pub header: Header,
    pub blocks: Vec<Block>,
    pub dna: Dna,
}
