extern crate blend_sdna;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use blend_sdna::sdna;
use std::collections::HashMap;

lazy_static! {
    //static ref POINTER_RE: regex::Regex = regex::Regex::new(r"\*([A-Za-z0-9]+)").unwrap();
    static ref POINTER_RE: regex::Regex = regex::Regex::new(r"([\*]+)([A-Za-z0-9]+)").unwrap();
    static ref POINTER_ARRAY_RE: regex::Regex = regex::Regex::new(r"\*([A-Za-z0-9]+)\[([0-9]+)\]").unwrap();
    static ref ARRAY_RE: regex::Regex = regex::Regex::new(r"([A-Za-z0-9]+)\[([0-9]+)\]").unwrap();
    static ref FN_PTR_RE: regex::Regex = regex::Regex::new(r"\(\*([A-Za-z0-9]+)\)\((.*)\)").unwrap();
}

#[derive(Debug, Eq, PartialEq)]
pub enum FieldFormat {
    Pointer,
    Value,
    FunctionPointer,
}

#[derive(Debug)]
pub struct FieldInstance<'a> {
    name: &'a str,
    original_name: &'a str,
    type_template: &'a sdna::Type,
    struct_template: Option<&'a sdna::StructureTemplate>,
    offset: usize,
    length: usize,
    format: FieldFormat,
    num_elements: usize,
}

pub struct Instance<'a> {
    pub code: Option<[u8; 4]>,
    addr: Option<u64>,
    blend: &'a sdna::Blend,
    type_template: &'a sdna::Type,
    struct_template: &'a sdna::StructureTemplate,
    fields: HashMap<&'a str, FieldInstance<'a>>,
    data: &'a [u8],
}

impl<'a> ::std::fmt::Display for Instance<'a> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        if let Some(addr) = self.addr {
            write!(fmt, "{} @{}", self.type_template.name, addr)?;
        } else {
            write!(fmt, "{}", self.type_template.name)?;
        }

        if let Some(code) = self.code {
            write!(fmt, " | code: {}", String::from_utf8_lossy(&code));
        }

        writeln!(fmt, "{{");

        let sorted_fields = {
            let mut sorted_fields = self.fields.iter().map(|v| *v.0).collect::<Vec<_>>();
            sorted_fields.sort();
            sorted_fields
        };

        for fname in sorted_fields.iter() {
            let f = &self.fields[fname];
            let pointer = if f.format == FieldFormat::Pointer {
                "*"
            } else {
                ""
            };
            let array = if f.num_elements > 1 {
                format!("[{}]", f.num_elements)
            } else {
                "".to_string()
            };

            write!(
                fmt,
                "\t{}{}{} {} ({})",
                pointer, f.type_template.name, array, f.name, f.original_name
            )?;

            if f.format == FieldFormat::Value {
                if f.type_template.name == "char" {
                    if f.num_elements > 1 {
                        write!(fmt, " = \"{}\"", self.get_string(f.name))?;
                    } else {
                        write!(fmt, " = \"{}\"", self.get_char(f.name))?;
                    }
                } else if f.type_template.name == "float" {
                    if f.num_elements == 1 {
                        write!(fmt, " = {}f", self.get_float(f.name))?;
                    } else {
                        write!(fmt, " = ")?;
                        let mut els = Vec::new();
                        for el in 0..f.num_elements {
                            els.push(self.get_float_at(f.name, el));
                        }
                        fmt.debug_list().entries(els.iter()).finish()?;
                    }
                } else if f.type_template.name == "int" {
                    if f.num_elements == 1 {
                        write!(fmt, " = {}", self.get_int(f.name))?;
                    } else {
                        write!(fmt, " = ")?;
                        let mut els = Vec::new();
                        for el in 0..f.num_elements {
                            els.push(self.get_int_at(f.name, el));
                        }
                        fmt.debug_list().entries(els.iter()).finish()?;
                    }
                } else if f.type_template.name == "short" {
                    if f.num_elements == 1 {
                        write!(fmt, " = {}", self.get_short_at(f.name, 0))?;
                    } else {
                        write!(fmt, " = ")?;
                        let mut els = Vec::new();
                        for el in 0..f.num_elements {
                            els.push(self.get_short_at(f.name, el));
                        }
                        fmt.debug_list().entries(els.iter()).finish()?;
                    }
                } else {
                    if f.type_template.name == "ID" {
                        write!(fmt, " = {}", self.get_instance(f.name));
                    }
                }
            } else if f.format == FieldFormat::Pointer {
                write!(fmt, " = @{:?}", self.get_ptr_at(f.name, 0))?;
            }

            writeln!(fmt, ";")?;
        }

        writeln!(fmt, "}}")
    }
}

impl<'a> Instance<'a> {
    pub fn type_is(&'a self, type_name: &'a str) -> bool {
        self.type_template.name == type_name
    }

    pub fn get_instance<T: AsRef<str>>(&'a self, field_name: T) -> Instance<'a> {
        self.fields
            .get(field_name.as_ref())
            .map(|field| {
                let struct_template = field
                    .struct_template
                    .expect("called get_instance on non-instance field");
                Instance {
                    code: None,
                    addr: None,
                    blend: &self.blend,
                    type_template: field.type_template,
                    struct_template: struct_template,
                    fields: Blend::create_fields(
                        &self.blend.dna,
                        &self.blend.header,
                        struct_template,
                    ),
                    data: &self.data[field.offset..field.length],
                }
            }).unwrap()
    }

    fn get_at(&'a self, field: &'a FieldInstance, index: usize) -> Option<&'a [u8]> {
        let size = field.length / field.num_elements;
        let min = field.offset + size * index;
        let max = field.offset + size + size * index;

        if min < self.data.len() && max <= self.data.len() {
            Some(&self.data[min..max])
        } else {
            //print!("[ERR] get_at ({} {})", field.type_template.name, field.name);
            //eprintln!("[ERR] get_at ({} {})", field.type_template.name, field.name);
            None
        }
    }

    pub fn get_char<T: AsRef<str>>(&'a self, name: T) -> u8 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_char on non-value field");
                }
                let data = self.get_at(f, 0).unwrap();
                assert!(data.len() == f.length / f.num_elements);
                let mut c = ::std::io::Cursor::new(data);
                //sdna::read_i32(&mut c, self.blend.header.endianness).ok()
                sdna::read_exact(&mut c, 1)
                    .ok()
                    .and_then(|v| v.iter().cloned().next())
                    .unwrap()
            }).unwrap()
    }
    pub fn get_short_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i16 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_short_at on non-value field");
                }
                let data = self.get_at(f, index).unwrap();
                assert!(data.len() == f.length / f.num_elements);
                let mut c = ::std::io::Cursor::new(data);
                sdna::read_i16(&mut c, self.blend.header.endianness)
                    .ok()
                    .unwrap()
            }).unwrap()
    }
    pub fn get_i16_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i16 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_int_at on non-value field");
                }
                let data = self.get_at(f, index).unwrap();
                assert!(data.len() == f.length / f.num_elements);
                let mut c = ::std::io::Cursor::new(data);
                sdna::read_i16(&mut c, self.blend.header.endianness)
                    .ok()
                    .unwrap()
            }).unwrap()
    }
    pub fn get_i16<T: AsRef<str>>(&'a self, name: T) -> i16 {
        self.get_i16_at(name, 0)
    }
    pub fn get_int_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> i32 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_int_at on non-value field");
                }
                let data = self.get_at(f, index).unwrap();
                assert!(data.len() == f.length / f.num_elements);
                let mut c = ::std::io::Cursor::new(data);
                sdna::read_i32(&mut c, self.blend.header.endianness)
                    .ok()
                    .unwrap()
            }).unwrap()
    }
    pub fn get_int<T: AsRef<str>>(&'a self, name: T) -> i32 {
        self.get_int_at(name, 0)
    }
    pub fn get_float_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> f32 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_float on non-value field");
                }
                let data = self.get_at(f, index).unwrap();
                assert!(data.len() == f.length / f.num_elements);
                let mut c = ::std::io::Cursor::new(data);
                sdna::read_f32(&mut c, self.blend.header.endianness)
                    .ok()
                    .unwrap()
            }).unwrap()
    }
    pub fn get_float<T: AsRef<str>>(&'a self, name: T) -> f32 {
        self.get_float_at(name, 0)
    }
    pub fn get_ptr<T: AsRef<str>>(&'a self, name: T) -> u64 {
        self.get_ptr_at(name, 0)
    }
    pub fn get_ptr_at<T: AsRef<str>>(&'a self, name: T, index: usize) -> u64 {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Pointer {
                    panic!("Called get_ptr on non-pointer field");
                }
                if let Some(data) = self.get_at(f, index) {
                    if data.len() != f.length / f.num_elements {
                        return Some(0);
                    }

                    /*if(f.type_template.name == "Link") {
                        let next_ptr = self.get_ptr_at("next", 0);
                        let prev_ptr = self.get_ptr_at("prev", 0);

                        if next_ptr != 0 {
                            return Some(next_ptr);
                        }
                        else {
                            return Some(prev_ptr);
                        }
                    }*/

                    //assert!();
                    let mut c = ::std::io::Cursor::new(data);
                    sdna::read_ptr(
                        &mut c,
                        self.blend.header.endianness,
                        self.blend.header.pointer_size,
                    ).ok()
                } else {
                    Some(0)
                }
            }).unwrap()
            .unwrap()
    }
    pub fn get_string<T: AsRef<str>>(&'a self, name: T) -> String {
        self.fields
            .get(name.as_ref())
            .map(|ref f| {
                if f.format != FieldFormat::Value {
                    panic!("Called get_string on non-value field");
                }

                if f.length as usize / f.num_elements != 1 {
                    panic!(
                        "Called get_string on field with incompatible type ({} {})",
                        f.type_template.name,
                        name.as_ref()
                    );
                }

                let mut final_offset = f.offset as usize + f.length as usize;

                for i in f.offset as usize..final_offset {
                    if &self.data[i] == &('\0' as u8) {
                        final_offset = i;
                        break;
                    }
                }

                String::from_utf8_lossy(&self.data[f.offset as usize..final_offset]).into()
            }).unwrap()
    }
}

pub struct Blend {
    memory: HashMap<u64, sdna::Block>,
    blend: sdna::Blend,
}

impl ::std::fmt::Display for Blend {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        for (addr, _) in &self.memory {
            println!("addr: {}", addr);

            if let Some(x) = self.try_get_instance(*addr) {
                writeln!(fmt, "{}", x)?;
            } else {
                writeln!(fmt, "[NULL]")?;
            }
        }
        Ok(())
    }
}

impl Blend {
    pub fn new<R: ::std::io::Read>(buf: &mut R) -> ::std::io::Result<Blend> {
        let mut blend = sdna::parse_blend_file(buf)?;

        let mut blocks = ::std::mem::replace(&mut blend.blocks, Vec::new());
        let k = blocks.drain(..);
        let j = k
            .map(|b| (b.header.old_memory_address, b))
            .collect::<HashMap<_, _>>();

        Ok(Blend {
            memory: j,
            blend: blend,
        })
    }

    fn create_fields<'a>(
        dna: &'a sdna::Dna,
        header: &'a sdna::Header,
        struct_template: &'a sdna::StructureTemplate,
    ) -> HashMap<&'a str, FieldInstance<'a>> {
        let mut fields = HashMap::new();
        let mut field_offset = 0;
        for field_template in &struct_template.fields {
            let mut field_name = dna.names[field_template.name_index].as_ref();
            let field_type_template = &dna.types[field_template.type_index];
            let mut num_elements = 1;
            let mut field_length = field_type_template.length as usize;
            let struct_template = dna
                .structures
                .iter()
                .filter(|s| s.type_index == field_template.type_index)
                .next();
            let mut field_format = FieldFormat::Value;

            if let Some(array_data) = POINTER_ARRAY_RE.captures_iter(&field_name).next() {
                field_name = &field_name[1..array_data[1].len() + 1];
                num_elements = (&array_data[2]).parse::<usize>().unwrap();
                field_length = header.pointer_size.length_in_bytes() as usize * num_elements;
                field_format = FieldFormat::Pointer;
            } else if let Some(array_data) = ARRAY_RE.captures_iter(&field_name).next() {
                field_name = &field_name[0..array_data[1].len()];
                num_elements = (&array_data[2]).parse::<usize>().unwrap();
                field_length *= num_elements;
            } else if let Some(pointer_name) = POINTER_RE.captures_iter(&field_name).next() {
                //if field_name == "**mat" {
                //    println!("{:?}", pointer_name);
                //    panic!()
                //}
                let x = pointer_name[1].len();
                let y = x + pointer_name[2].len();

                if x > 1 {
                    //println!(">>> {}", field_name);
                }

                field_name = &field_name[x..y];
                field_length = header.pointer_size.length_in_bytes() as usize;
                field_format = FieldFormat::Pointer;
            } else if let Some(_fn_ptr_data) = FN_PTR_RE.captures_iter(&field_name).next() {
                panic!("fn ptr")
            }

            fields.insert(
                field_name,
                FieldInstance {
                    name: field_name,
                    original_name: dna.names[field_template.name_index].as_ref(),
                    type_template: field_type_template,
                    struct_template: struct_template,
                    offset: field_offset,
                    length: field_length,
                    format: field_format,
                    num_elements: num_elements,
                },
            );
            field_offset += field_length;
        }

        fields
    }

    pub fn get_instance_count(&self, address: u64) -> u32 {
        self.memory
            .get(&address)
            .map(|block| block.header.count)
            .unwrap()
    }

    pub fn get_instance<'a>(&'a self, address: u64) -> Instance<'a> {
        self.get_instance_at(address, 0)
    }

    pub fn try_get_instance<'a>(&'a self, address: u64) -> Option<Instance<'a>> {
        self.memory.get(&address).and_then(|block| {
            let instance = self.block_to_instance(block, 0);
            if instance.type_template.name == "Link" {
                let next = instance.get_ptr_at("next", 0);
                let prev = instance.get_ptr_at("prev", 0);

                if next != 0 {
                    return self.try_get_instance(next);
                } else if prev != 0 {
                    return self.try_get_instance(prev);
                }
            }
            Some(instance)
        })
    }

    pub fn get_instance_by_code<'a>(&'a self, code: &'a [u8; 4]) -> Instance<'a> {
        self.memory
            .iter()
            .filter(|&(_, block)| &block.header.code == code)
            .next()
            .map(|(_, block)| self.block_to_instance(block, 0))
            .unwrap()
    }

    pub fn get_instances_by_code<'a>(&'a self, code: &'a [u8; 4]) -> Vec<Instance<'a>> {
        self.memory
            .iter()
            .filter(|&(_, block)| {
                if &block.header.code == code {
                    //println!("{}", String::from_utf8_lossy(&block.header.code));
                    true
                } else {
                    false
                }
            }).map(|(_, block)| self.block_to_instance(block, 0))
            .collect()
    }

    pub fn get_instance_at<'a>(&'a self, address: u64, index: usize) -> Instance<'a> {
        self.memory
            .get(&address)
            .map(|block| {
                let instance = self.block_to_instance(block, index);
                if instance.type_template.name == "Link" {
                    let next = instance.get_ptr_at("next", 0);
                    let prev = instance.get_ptr_at("prev", 0);

                    if next != 0 {
                        return self.get_instance(next);
                    } else if prev != 0 {
                        return self.get_instance(prev);
                    }
                }
                instance
            }).unwrap()
    }

    fn block_to_instance<'a>(&'a self, block: &'a sdna::Block, index: usize) -> Instance<'a> {
        let struct_template = &self.blend.dna.structures[block.header.sdna_index];
        let type_template = &self.blend.dna.types[struct_template.type_index];

        let initial_index = index * type_template.length as usize;

        Instance {
            code: Some(block.header.code),
            addr: Some(block.header.old_memory_address),
            blend: &self.blend,
            type_template: type_template,
            struct_template: struct_template,
            data: &block.data[initial_index..],
            fields: Blend::create_fields(&self.blend.dna, &self.blend.header, &struct_template),
        }
    }
}
