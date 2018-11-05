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
    count: Option<usize>,
    is_primitive: bool,
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
    pub code: Option<[u8; 4]>,
    struct_type: &'a (String, u16),
    fields: HashMap<&'a str, FieldInstance<'a>>, //Vec<FieldInstance<'a>>
    block: Option<&'a Block>,
    struct_template_index: usize,
}

pub fn parse_i8(slice: &[u8], endianness: Endianness) -> i8 {
    let mut rdr = Cursor::new(slice);
    rdr.read_i8().unwrap()
}

pub fn parse_u8(slice: &[u8], endianness: Endianness) -> u8 {
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

impl<'a> StructInstance<'a> {
    fn get_at(&'a self, field: &'a FieldInstance, index: usize) -> Option<&'a [u8]> {
        unimplemented!()
    }

    pub fn get_char<T: AsRef<str>>(&'a self, name: T) -> u8 {
        unimplemented!()
    }

    pub fn get_i8_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i8 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<i8>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_i8(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_i8<T: AsRef<str>>(&'a self, name: T) -> i8 {
        self.get_i8_at(name, 0)
    }

    pub fn get_u8_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> u8 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<u8>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_u8(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_u8<T: AsRef<str>>(&'a self, name: T) -> u8 {
        self.get_u8_at(name, 0)
    }

    pub fn get_u16_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> u16 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<u16>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_u16(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_u16<T: AsRef<str>>(&'a self, name: T) -> u16 {
        self.get_u16_at(name, 0)
    }

    pub fn get_i16_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i16 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<i16>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_i16(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_i16<T: AsRef<str>>(&'a self, name: T) -> i16 {
        self.get_i16_at(name, 0)
    }

    pub fn get_i32_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i32 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<i32>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_i32(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_i32<T: AsRef<str>>(&'a self, name: T) -> i32 {
        self.get_i32_at(name, 0)
    }

    pub fn get_f32_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> f32 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<f32>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_f32(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_f32<T: AsRef<str>>(&'a self, name: T) -> f32 {
        self.get_f32_at(name, 0)
    }

    pub fn get_f64_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> f64 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<f64>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_f64(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_f64<T: AsRef<str>>(&'a self, name: T) -> f64 {
        self.get_f64_at(name, 0)
    }

    pub fn get_i64_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i64 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<i64>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_i64(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_i64<T: AsRef<str>>(&'a self, name: T) -> i64 {
        self.get_i64_at(name, 0)
    }

    pub fn get_u64_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> u64 {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<u64>();
        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));
        parse_u64(
            &field.data[index * size..index * size + size],
            self.endianness,
        )
    }

    pub fn get_u64<T: AsRef<str>>(&'a self, name: T) -> u64 {
        self.get_u64_at(name, 0)
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

    pub fn get_instance<T: AsRef<str>>(&'a self, name: T) -> StructInstance<'a> {
        let field = self.fields.get(name.as_ref()).unwrap();

        let (i, _) = field
            .blend
            .struct_templates
            .iter()
            .enumerate()
            .filter(|s| {
                if s.1.struct_type_index == field.field_type_index {
                    return true;
                }
                false
            })
            .next()
            .unwrap();

        field.blend.data_to_struct(&field.data, i)
    }

    pub fn get_ptr_instance_list<T: AsRef<str>>(&'a self, name: T) -> Vec<StructInstance<'a>> {
        let field = self.fields.get(name.as_ref()).unwrap();

        let mut ret = Vec::new();

        println!("{:?}", field);

        if let Some((j, _)) = field
            .blend
            .struct_templates
            .iter()
            .enumerate()
            .filter(|s| {
                if s.1.struct_type_index == field.field_type_index {
                    return true;
                }
                false
            })
            .next()
        {
            for i in 0..field.data.len() / 8 {
                let ptr = parse_u64(
                    &field.data[i * 8..i * 8 + 8],
                    field.blend.blend.header.endianness,
                );

                if ptr == 0 {
                    panic!("null ptr");
                }

                let block = self
                    .blend
                    .blocks
                    .iter()
                    .filter(|b| b.header.old_memory_address == ptr)
                    .next()
                    .unwrap();

                let instance = field.blend.block_to_struct(&block, j);
                ret.push(instance);
            }
        } else {
            panic!("weird!");
        }

        ret
    }

    pub fn deref_link<T: AsRef<str>>(&'a self, name: T) -> Vec<StructInstance<'a>> {
        let ptr = self.get_ptr(name.as_ref());
        let field = self.fields.get(name.as_ref()).unwrap();

        let block = self
            .blend
            .blocks
            .iter()
            .filter(|b| b.header.old_memory_address == ptr)
            .next()
            .unwrap();

        let mut ret = Vec::new();

        if let Some((j, _)) = field
            .blend
            .struct_templates
            .iter()
            .enumerate()
            .filter(|s| {
                if s.1.struct_type_index == field.field_type_index {
                    return true;
                }
                false
            })
            .next()
        {
            for i in 0..block.data.len() / 8 {
                let ptr = parse_u64(
                    &block.data[i * 8..i * 8 + 8],
                    field.blend.blend.header.endianness,
                );

                if ptr == 0 {
                    panic!("null ptr");
                }

                let block = self
                    .blend
                    .blocks
                    .iter()
                    .filter(|b| b.header.old_memory_address == ptr)
                    .next()
                    .unwrap();

                let instance = field.blend.block_to_struct(&block, j);
                ret.push(instance);
            }
        } else {
            panic!("weird link deref");
        }

        ret
    }

    pub fn deref_instance<T: AsRef<str>>(&'a self, name: T) -> Option<StructInstance<'a>> {
        self.deref_instance_at(name, 0)
    }

    pub fn deref_instance_at<T: AsRef<str>>(
        &'a self,
        name: T,
        index: usize,
    ) -> Option<StructInstance<'a>> {
        let ptr = self.get_ptr(name.as_ref());
        let field = self.fields.get(name.as_ref()).unwrap();

        let block = self
            .blend
            .blocks
            .iter()
            .filter(|b| b.header.old_memory_address == ptr)
            .next()?;

        if index >= block.header.count as usize {
            panic!("invalid index");
        }

        let start_i = index * (block.data.len() / block.header.count as usize);
        let end_i = start_i + (block.data.len() / block.header.count as usize);

        if block.header.code[2..=3] == [0u8, 0] {
            let mut instance = field.blend.data_to_struct(
                &block.data[start_i..end_i],
                block.header.sdna_index as usize,
            );
            instance.block = Some(block);
            instance.code = Some(block.header.code);

            Some(instance)
        } else {
            if let Some((i, _)) = field
                .blend
                .struct_templates
                .iter()
                .enumerate()
                .filter(|s| {
                    if s.1.struct_type_index == field.field_type_index {
                        return true;
                    }
                    false
                })
                .next()
            {
                if field.indirection_count == 1 {
                    let mut instance = field.blend.data_to_struct(&block.data[start_i..end_i], i);
                    instance.block = Some(block);
                    instance.code = Some(block.header.code);

                    Some(instance)
                } else {
                    let template = block.header.sdna_index as usize;

                    let mut instance = field
                        .blend
                        .data_to_struct(&block.data[start_i..end_i], template);
                    instance.block = Some(block);
                    instance.code = Some(block.header.code);

                    Some(instance)
                }
            } else {
                panic!("{:?}", field);
            }
        }
    }

    pub fn get_string<T: AsRef<str>>(&'a self, name: T) -> String {
        let field = self.fields.get(name.as_ref()).unwrap();
        let size = std::mem::size_of::<u8>();

        assert_eq!(field.data.len(), size * field.count.unwrap_or(1));

        String::from_utf8_lossy(&field.data[0..size * field.count.unwrap_or(1)]).into()
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

    pub fn data_to_struct<'a>(
        &'a self,
        data: &'a [u8],
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
            let mut is_array = false;

            if let Ok(Some(data)) = BLEND_VARIABLE.captures(&field_name) {
                let is_fn_ptr = if let Some(parens) = data.at(1) {
                    parens == "(("
                } else {
                    false
                };

                if !is_fn_ptr {
                    let asterisks = data.at(2).unwrap();
                    variable_name = data.at(3).expect("no variable name");

                    is_array = if let Some(array_count) = data.at(4) {
                        array_count != ""
                    } else {
                        false
                    };

                    //println!("{} {:?}", variable_name, data.at(4));

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

            let is_primitive = match &field_type.0[..] {
                "int" | "char" | "uchar" | "short" | "ushort" | "float" | "double" | "long"
                | "ulong" | "int64_t" | "uint64_t" | "void" => true,
                _ => false,
            };

            if field.field_type_index < 12 && !is_primitive {
                panic!("algo errado {:?}", field_type);
            }

            struct_fields.push(FieldInstance {
                blend: &self,
                field_type,
                field_type_index: field.field_type_index,
                field_name,
                indirection_count,
                count: if is_array { Some(count) } else { None },
                name: variable_name,
                data: if struct_template_index != 0 {
                    &data[data_start..data_start + data_len]
                } else {
                    &data[data_start..]
                },
                is_primitive,
            });
            data_start += data_len;
        }

        if data_start != struct_type.1 as usize {
            println!(
                "{} (size {} {}) (code)",
                struct_type.0, struct_type.1, data_start,
            )
        }

        StructInstance {
            blend: &self.blend,
            struct_type: &self.dna.types[struct_template.struct_type_index],
            struct_template_index: struct_template_index,
            fields: struct_fields.into_iter().map(|f| (f.name, f)).collect(),
            endianness: self.blend.header.endianness,
            pointer_size: self.blend.header.pointer_size,
            code: None,
            block: None,
        }
    }

    pub fn block_to_struct<'a>(
        &'a self,
        block: &'a Block,
        struct_template_index: usize,
    ) -> StructInstance<'a> {
        let mut s = self.data_to_struct(&block.data, struct_template_index);
        s.code = Some(block.header.code);
        s.block = Some(&block);
        s
    }
}

use std::collections::HashSet;

fn print_struct_instance<'a>(
    done: &'a mut HashSet<u64>,
    tab_count: usize,
    ob: &'a StructInstance<'a>,
) -> String {
    let tabs = std::iter::repeat("  ").take(tab_count).collect::<String>();

    if let Some(block) = ob.block.as_ref() {
        if done.contains(&block.header.old_memory_address) {
            return format!("");
        }

        done.insert(block.header.old_memory_address);
    }

    let mut ret = String::new();
    ret.push_str(&format!(
        "{}{} {} {}\n",
        if ob.code.is_some() {
            tabs.clone()
        } else {
            String::new()
        },
        ob.struct_type.0,
        if let Some(code) = ob.code.as_ref() {
            String::from_utf8_lossy(code)
        } else {
            Default::default()
        },
        if let Some(block) = ob.block.as_ref() {
            format!("{}", block.header.old_memory_address)
        } else {
            format!("")
        }
    ));
    for (name, f) in &ob.fields {
        if f.is_primitive && f.indirection_count == 0 {
            if f.count.is_none() {
                match &f.field_type.0[..] {
                    "int" => {
                        ret.push_str(&format!(
                            "  {}[{}] {} {} = {}\n",
                            tabs,
                            f.field_name,
                            f.field_type.0,
                            name,
                            &ob.get_i32(name)
                        ));
                    }
                    "char" => {
                        ret.push_str(&format!(
                            "  {}[{}] {} {} = {}\n",
                            tabs,
                            f.field_name,
                            f.field_type.0,
                            name,
                            &ob.get_i8(name)
                        ));
                    }
                    /*"uchar" => (),
                    "short" => (),
                    "ushort" => (),
                    "float" => (),
                    "double" => (),
                    "long" => (),
                    "ulong" => (),
                    "int64_t" => (),
                    "uint64_t" => (),
                    "void" => (),*/
                    _ => {
                        ret.push_str(&format!(
                            "  {}[{}] {} {} = [NOT_IMPLEMENTED]\n",
                            tabs, f.field_name, f.field_type.0, name
                        ));
                    }
                }
            } else {
                match &f.field_type.0[..] {
                    "char" => {
                        ret.push_str(&format!(
                            "  {}[{}] {} {} = {}\n",
                            tabs,
                            f.field_name,
                            f.field_type.0,
                            name,
                            ob.get_string(f.name)
                        ));
                    }
                    _ => {
                        ret.push_str(&format!(
                            "  {}[{}] {} {} = [NOT_IMPLEMENTED]\n",
                            tabs, f.field_name, f.field_type.0, name
                        ));
                    }
                }
            }
        } else if !f.is_primitive && f.indirection_count == 0 {
            ret.push_str(&format!(
                "  {}[{}] {} {} = ",
                tabs, f.field_name, f.field_type.0, name
            ));
            ret.push_str(&print_struct_instance(
                done,
                tab_count + 1,
                &ob.get_instance(f.name),
            ));
        } else if !f.is_primitive && f.indirection_count > 0 {
            let ptr = ob.get_ptr(f.name);
            ret.push_str(&format!(
                "  {}[{}] {} {} = ",
                tabs, f.field_name, f.field_type.0, name
            ));

            if ptr != 0 {
                if let Some(instance) = ob.deref_instance(f.name) {
                    if let Some(block) = instance.block {
                        if block.header.sdna_index == 0 {
                            ret.push_str(&format!("{} (LINK) ", ptr));
                            for i in 0..block.data.len() / 8 {
                                ret.push_str(&format!(
                                    "{} ",
                                    parse_u64(
                                        &block.data[i * 8..i * 8 + 8],
                                        f.blend.blend.header.endianness
                                    )
                                ));
                            }
                        } else {
                            let struct_str = print_struct_instance(done, tab_count + 1, &instance);

                            if struct_str == "" {
                                ret.push_str(&format!("{} (repeated)", ptr));
                            } else {
                                ret.push_str(&struct_str);
                            }
                        }
                    } else {
                        ret.push_str(&format!("{} (aba)", ptr));
                    }
                } else {
                    ret.push_str(&format!("{} (link?)", ptr));
                }
            } else {
                ret.push_str(&format!("{}", ptr));
            }

            ret.push_str(&format!("\n"));
        } else if f.is_primitive && f.indirection_count > 0 {
            let ptr = ob.get_ptr(f.name);

            ret.push_str(&format!(
                "  {}>>[{}] {} {} = {}\n",
                tabs, f.field_name, f.field_type.0, name, ptr
            ));
        }
    }

    ret
}

pub fn main() {
    use std::fs::File;
    use std::io::{Read, Write};

    let mut file = File::open("/home/lucas/projects/leaf/assets/simple.blend").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let blend = Blend::new(&buffer[..]);

    let mut buffer = File::create("hello2.txt").unwrap();

    let mut done = HashSet::new();

    for ob in blend.test_all() {
        //let data: Vec<_> = print_struct_instance(&mut done, 0, &ob).bytes().collect();
        //buffer.write(&data[..]).unwrap();
    }

    for ob in blend.get_by_code([b'O', b'B']) {
        println!("{}", ob.get_instance("id").get_string("name"));
        let data = ob.deref_instance("data").unwrap();

        //println!("{}", String::from_utf8_lossy(&.unwrap()));

        if data.code == Some([b'M', b'E', 0, 0]) {
            let d1: Vec<_> = print_struct_instance(&mut done, 0, &ob).bytes().collect();
            buffer.write(&d1[..]).unwrap();

            let d2: Vec<_> = print_struct_instance(&mut done, 0, &data).bytes().collect();
            buffer.write(&d2[..]).unwrap();

            let mat = data.deref_link("mat");
            let d3: Vec<_> = print_struct_instance(&mut done, 0, &mat[0])
                .bytes()
                .collect();
            buffer.write(&d3[..]).unwrap();

            for mat in mat {
                let nodetree = mat.deref_instance("nodetree").unwrap();

                let d4: Vec<_> = print_struct_instance(&mut done, 0, &nodetree)
                    .bytes()
                    .collect();
                buffer.write(&d4[..]).unwrap();

                let nodes = nodetree.get_ptr_instance_list("nodes");

                for node in nodes {
                    println!("{:?}", node);

                    let d5: Vec<_> = print_struct_instance(&mut done, 0, &node).bytes().collect();
                    buffer.write(&d5[..]).unwrap();
                }
            }
        }
    }
}
