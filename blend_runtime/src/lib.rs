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
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use fancy_regex::Regex;
use num::FromPrimitive;
use std::collections::HashMap;
use std::io::Cursor;

lazy_static! {
    static ref BLEND_VARIABLE: Regex =
        //Regex::new(r"([\(]*)([\*]*)([a-zA-Z0-9_]*)(?:(?:\))|\[*([0-9]*)\]*)").unwrap();
        Regex::new(r"([\(]*)([\*]*)([a-zA-Z0-9_]*)(?:(?:\))|\[*([0-9]*)\]*(?:\[([0-9]*)\])*)").unwrap();

}

#[derive(Debug)]
pub struct FieldTemplate {
    field_type_index: usize,
    field_name_index: usize,
}

#[derive(Debug)]
pub struct StructTemplate {
    struct_type_index: usize,
    struct_fields: Vec<FieldTemplate>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FieldInstance<'a> {
    #[derivative(Debug = "ignore")]
    blend: &'a Blend,
    field_type: &'a (String, u16),
    field_type_index: usize,
    field_name: &'a String,
    indirection_count: u8,
    name: &'a str,
    #[derivative(Debug = "ignore")]
    data: &'a [u8],
    count: usize,
    /*data_start: usize,
    data_len: usize,*/
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct StructInstance<'a> {
    //struct_template: &'a StructTemplate,
    #[derivative(Debug = "ignore")]
    blend: &'a ParsedBlend,
    endianness: Endianness,
    pointer_size: PointerSize,
    pub code: [u8; 4],
    struct_type: &'a (String, u16),
    fields: HashMap<&'a str, FieldInstance<'a>>, //Vec<FieldInstance<'a>>
}

pub fn parse_f32(slice: &[u8], endianness: Endianness) -> f32 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_f32::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_f32::<BigEndian>().unwrap(),
    }
}

pub fn parse_u32(slice: &[u8], endianness: Endianness) -> u32 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_u32::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_u32::<BigEndian>().unwrap(),
    }
}

pub fn parse_u64(slice: &[u8], endianness: Endianness) -> u64 {
    let mut rdr = Cursor::new(slice);
    match endianness {
        Endianness::LittleEndian => rdr.read_u64::<LittleEndian>().unwrap(),
        Endianness::BigEndian => rdr.read_u64::<BigEndian>().unwrap(),
    }
}

impl<'a> StructInstance<'a> {
    pub fn get_instance<T: AsRef<str>>(&'a self, field_name: T) -> StructInstance<'a> {
        unimplemented!()
    }

    fn get_at(&'a self, field: &'a FieldInstance, index: usize) -> Option<&'a [u8]> {
        unimplemented!()
    }

    pub fn get_char<T: AsRef<str>>(&'a self, name: T) -> u8 {
        unimplemented!()
    }

    pub fn get_i16_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i16 {
        unimplemented!()
    }

    pub fn get_i16<T: AsRef<str>>(&'a self, name: T) -> i16 {
        self.get_i16_at(name, 0)
    }

    pub fn get_i32_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i32 {
        unimplemented!()
    }

    pub fn get_i32<T: AsRef<str>>(&'a self, name: T) -> i32 {
        self.get_i32_at(name, 0)
    }

    pub fn get_f32_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> f32 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<f32>();
        assert_eq!(field.data.len(), size * field.count);
        parse_f32(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_f32<T: AsRef<str>>(&'a self, name: T) -> f32 {
        self.get_f32_at(name, 0)
    }

    pub fn get_ptr_instance_count<T: AsRef<str>>(&'a self, name: T) -> usize {
        let ptr: u64 = self.get_ptr(name);

        self.blend
            .blocks
            .iter()
            .filter(|b| b.header.old_memory_address == ptr)
            .next()
            .map(|b| b.header.count as usize)
            .unwrap()
    }

    pub fn get_ptr<T: AsRef<str>>(&'a self, name: T) -> u64 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = self.pointer_size.bytes_num();

        assert_eq!(field.data.len(), size);

        match self.pointer_size {
            PointerSize::Bits32 => parse_u32(field.data, self.endianness) as u64,
            PointerSize::Bits64 => parse_u64(field.data, self.endianness),
        }
    }

    /*pub fn get_ptr_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> u64 {
        unimplemented!()
    }*/

    pub fn deref_instance<T: AsRef<str>>(&'a self, name: T) -> StructInstance<'a> {
        self.deref_instance_at(name, 0)
    }

    pub fn deref_instance_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> StructInstance<'a> {
        let mut ptr = self.get_ptr(name.as_ref());
        let field = self.fields.get(name.as_ref()).unwrap();

        println!("ptr {}", ptr);

        let mut block = self
            .blend
            .blocks
            .iter()
            .filter(|b| b.header.old_memory_address == ptr)
            .next()
            .unwrap();
        println!("{:?}", field);
        println!("{:?} {:?}", block, block.data);

        for _ in 1..field.indirection_count {
            println!(">>>");
            let next_ptr = match self.pointer_size {
                PointerSize::Bits32 => parse_u32(&block.data, self.endianness) as u64,
                PointerSize::Bits64 => parse_u64(&block.data, self.endianness),
            };

            println!("next {}", next_ptr);

            block = self
                .blend
                .blocks
                .iter()
                .filter(|b| b.header.old_memory_address == next_ptr)
                .next()
                .unwrap();
        }

        println!(
            "ends 00 {}",
            block.header.code[2] == 0 && block.header.code[3] == 0
        );

        if field.field_type.1 == 0 {
            let instance = field
                .blend
                .block_to_struct(&block, block.header.sdna_index as usize);

            println!("{:?}", instance.struct_type);

            instance
        } else {
            //println!("{:?}", field.blend.dna);

            let (i, struct_template) = field
                .blend
                .struct_templates
                .iter()
                .enumerate()
                .filter(|s| {
                    if s.1.struct_type_index == field.field_type_index {
                        println!("\t{}", s.0 as usize);
                        return true;
                    }
                    false
                })
                .next()
                .unwrap();

            /*println!(
                ">? {} {:?}",
                i,
                field.blend.dna.types[field.blend.struct_templates[i].struct_type_index] //i, field.blend.dna.types[field.field_type_index]
            );*/

            field.blend.block_to_struct(&block, i)
        }
    }

    pub fn get_string<T: AsRef<str>>(&'a self, name: T) -> String {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Blend {
    blend: ParsedBlend,
    dna: Dna,
    struct_templates: Vec<StructTemplate>,
    //memory: HashMap<u64, Block>,
}

impl Blend {
    pub fn new(data: &[u8]) -> Blend {
        let blend = ParsedBlend::new(data).unwrap();

        let dna = {
            let dna_block = &blend.blocks[blend.blocks.len() - 1];
            Dna::from_sdna_block(
                dna_block,
                blend.header.endianness,
                blend.header.pointer_size,
            )
            .unwrap()
        };

        let mut struct_templates: Vec<StructTemplate> = Vec::new();

        for structure in &dna.structs {
            let mut struct_fields = Vec::new();

            for field in &structure.1 {
                struct_fields.push(FieldTemplate {
                    field_type_index: /*types[*/ field.0 as usize,
                    field_name_index: /*names[*/ field.1 as usize,
                })
            }

            struct_templates.push(StructTemplate {
                struct_type_index: /*types[*/ structure.0 as usize,
                struct_fields,
            });
        }

        Blend {
            blend,
            dna,
            struct_templates,
        }
    }

    fn test_all(&self) -> impl Iterator<Item = StructInstance> {
        self.blend
            .blocks
            .iter()
            .filter(|b| b.header.code == [b.header.code[0], b.header.code[1], 0, 0])
            .map(|b| self.block_to_struct(b, b.header.sdna_index as usize))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn get_by_code(&self, code: [u8; 2]) -> impl Iterator<Item = StructInstance> {
        self.blend
            .blocks
            .iter()
            .filter(|b| b.header.code == [code[0], code[1], 0, 0])
            .map(|b| self.block_to_struct(b, b.header.sdna_index as usize))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn block_to_struct<'a>(
        &'a self,
        block: &'a Block,
        struct_template_index: usize,
    ) -> StructInstance<'a> {
        let struct_template = &self.struct_templates[struct_template_index];
        let struct_type = &self.dna.types[struct_template.struct_type_index];

        let mut struct_fields = Vec::new();
        let mut data_start = 0usize;

        for field in &struct_template.struct_fields {
            let field_type = &self.dna.types[field.field_type_index];
            let field_name = &self.dna.names[field.field_name_index];

            let mut data_len;
            let mut variable_name = "";
            let mut count = 1;
            let mut indirection_count = 0;

            if let Ok(Some(data)) = BLEND_VARIABLE.captures(&field_name) {
                let is_fn_ptr = if let Some(parens) = data.at(1) {
                    parens == "(("
                } else {
                    false
                };

                if !is_fn_ptr {
                    let asterisks = data.at(2).unwrap();
                    variable_name = data.at(3).expect("no variable name");
                    let array_count: u16 = data.at(4).map(|c| c.parse().unwrap_or(1)).unwrap_or(1);
                    let array_count2: u16 = data.at(5).map(|c| c.parse().unwrap_or(1)).unwrap_or(1);

                    if asterisks.len() <= 255 {
                        indirection_count = asterisks.len() as u8;
                    } else {
                        panic!("number of indirections too high");
                    }

                    if asterisks.len() > 0 {
                        data_len = self.blend.header.pointer_size.bytes_num()
                    } else {
                        count = array_count as usize * array_count2 as usize;
                        data_len =
                            field_type.1 as usize * array_count as usize * array_count2 as usize;
                    }
                } else {
                    println!("{} | FN_PTR", field_name);
                    data_len = self.blend.header.pointer_size.bytes_num()
                }
            } else {
                panic!("unexpected field name: {}", field_name);
            }

            struct_fields.push(FieldInstance {
                blend: &self,
                field_type,
                field_type_index: field.field_type_index,
                field_name,
                indirection_count,
                count,
                name: variable_name,
                data: &block.data[data_start..data_start + data_len],
            });
            data_start += data_len;
        }

        if data_start != struct_type.1 as usize {
            println!(
                "{} (size {} {}) (code {})",
                struct_type.0,
                struct_type.1,
                data_start,
                String::from_utf8_lossy(&block.header.code)
            )
        }

        StructInstance {
            blend: &self.blend,
            struct_type: &self.dna.types[struct_template.struct_type_index],
            fields: struct_fields.into_iter().map(|f| (f.name, f)).collect(),
            endianness: self.blend.header.endianness,
            pointer_size: self.blend.header.pointer_size,
            code: block.header.code,
        }
    }
}

pub fn main() {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open("/home/lucas/projects/leaf/assets/simple.blend").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let blend = Blend::new(&buffer[..]);

    for ob in blend.get_by_code([b'O', b'B']) {
        if ob.get_ptr("mat") != 0 {
            println!("{} ({})", ob.struct_type.0, ob.struct_type.1);

            for (_, f) in &ob.fields {
                if f.field_name == "**mat" {
                    println!("\t{} {} ({})", f.field_type.0, f.field_name, f.data.len());
                }
            }

            println!(
                "float: {} {} {}",
                ob.get_f32_at("loc", 0),
                ob.get_f32_at("loc", 1),
                ob.get_f32_at("loc", 2)
            );

            ob.deref_instance("data");

            ob.deref_instance("mat");
        }
    }
}
