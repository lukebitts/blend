extern crate blend_parse;
extern crate blend_sdna;
extern crate fancy_regex;
#[macro_use]
extern crate lazy_static;
extern crate byteorder;
extern crate num;
#[macro_use]
extern crate derivative;

use blend_parse::{Blend as ParsedBlend, Block, Endianness, PointerSize};
use blend_sdna::Dna;

pub enum Field {
    Value,
    PointerToStruct,
    PointerToPointer,
}

pub enum Instance {
    Value,
    Struct,
}

pub fn main() {
    use std::fs::File;
    use std::io::{Read, Write};

    let mut file = File::open("/home/lucas/projects/leaf/assets/simple.blend").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let blend = ParsedBlend::new(&data[..]).unwrap();

    let dna = {
        let dna_block = &blend.blocks[blend.blocks.len() - 1];
        Dna::from_sdna_block(
            dna_block,
            blend.header.endianness,
            blend.header.pointer_size,
        )
        .unwrap()
    };

    let mut struct_templates: Vec<_> = Vec::new();

    for structure in &dna.structs {
        let mut struct_fields = Vec::new();

        for field in &structure.1 {
            struct_fields.push((
                /*field_type_index: */ field.0 as usize,
                /*field_name_index: */ field.1 as usize,
            ))
        }

        struct_templates.push((
            /*struct_type_index:*/ structure.0 as usize,
            struct_fields,
        ));
    }
}
