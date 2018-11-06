extern crate blend_parse;
extern crate blend_sdna;
extern crate fancy_regex;
#[macro_use]
extern crate lazy_static;
extern crate byteorder;
extern crate num;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate nom;

mod field_parser;

use blend_parse::{Blend as ParsedBlend, Block, Endianness, PointerSize};
use blend_sdna::Dna;
use field_parser::{parse_field, FieldInfo};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub info: FieldInfo,
    pub type_index: u16,
    pub data_start: usize,
    pub data_len: usize,
}

pub fn main() {
    use std::fs::File;
    use std::io::{Read, Write};

    //let r1 = property("*valor[1][2]");
    //println!("{:?}", r1);

    /*let mut buffer = File::create("hello2.txt").unwrap();

    let mut file = File::open("assets/simple.blend").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let blend = ParsedBlend::new(&data[..]).unwrap();

    let dna = {
        let dna_block = &blend.blocks[blend.blocks.len() - 1];
        Dna::from_sdna_block(
            dna_block,
            blend.header.endianness,
            blend.header.pointer_size,
        ).unwrap()
    };

    for (type_index, struct_fields) in &dna.structs {
        for (field_type_index, field_name_index) in struct_fields {
            let field_name = &dna.names[*field_name_index as usize];
            if let Ok((_, field)) = field(field_name) {
                let print_data: Vec<u8> =
                    format!("{} ({}, {:?})\n", field_name, field.name, field.info)
                        .bytes()
                        .collect();
                buffer.write(&print_data[..]).unwrap();
            } else {
                panic!("field name could not be parsed")
            }
        }
    }*/

    let mut file = File::open("assets/simple.blend").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let blend = ParsedBlend::new(&data[..]).unwrap();

    let dna = {
        let dna_block = &blend.blocks[blend.blocks.len() - 1];
        Dna::from_sdna_block(
            dna_block,
            blend.header.endianness,
            blend.header.pointer_size,
        ).unwrap()
    };

    let mut templates: HashMap<u16, _> = HashMap::new();

    for (struct_type_index, struct_fields) in &dna.structs {
        let (_struct_type_name, struct_type_bytes_len) = &dna.types[*struct_type_index as usize];
        let mut fields = Vec::new();

        let mut data_start = 0;
        for (field_type_index, field_name_index) in struct_fields {
            let (_field_type_name, field_type_bytes_len) = &dna.types[*field_type_index as usize];
            let field_full_name = &dna.names[*field_name_index as usize];

            let is_primitive = *field_type_index < 12;
            let (_, (field_name, field_info)) =
                parse_field(field_full_name).expect("field name could not be parsed");

            let field_bytes_len = match field_info {
                FieldInfo::Pointer { .. } | FieldInfo::FnPointer => {
                    blend.header.pointer_size.bytes_num()
                }
                FieldInfo::PointerArray1D { len } => blend.header.pointer_size.bytes_num() * len,
                FieldInfo::ValueArray1D { len } => *field_type_bytes_len as usize * len,
                FieldInfo::ValueArray2D { len1, len2 } => {
                    *field_type_bytes_len as usize * len1 * len2
                }
                FieldInfo::Value => *field_type_bytes_len as usize,
            };

            fields.push(Field {
                name: String::from(field_name),
                info: field_info,
                type_index: *field_type_index,
                data_start,
                data_len: field_bytes_len,
            });

            data_start += field_bytes_len;
        }

        assert_eq!(*struct_type_bytes_len as usize, data_start);

        templates.insert(*struct_type_index, fields);
    }

    for block in &blend.blocks {
        if block.header.code[0..=1] == [b'M', b'E'] && block.header.code[2..=3] == [0, 0] {
            let (struct_type_index, _) = &dna.structs[block.header.sdna_index as usize];
            //let struct_type_name = &dna.types[*struct_type_index as usize].0;
            let struct_template = &templates[struct_type_index];

            println!(
                "block {} {:#?}",
                String::from_utf8_lossy(&block.header.code[0..=1]),
                struct_template,
            );
        }
    }
}
