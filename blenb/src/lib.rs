pub mod parser;
pub mod sdna;

use crate::parser::field_parser::{parse_field, FieldInfo};
use crate::parser::primitive_parsers::BlendPrimitive;
use crate::parser::struct_parser::FieldTemplate;
use crate::parser::primitive_parsers::*;
use crate::parser::{Blend as ParsedBlend, Block, Header as BlendHeader, PointerSize};
use crate::sdna::Dna;
use linked_hash_map::LinkedHashMap; 
use std::io::Read;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::path::Path;
use std::collections::{VecDeque, HashSet};

#[derive(Clone)]
pub enum InstanceDataFormat<'a> {
    Block(&'a Block),
    Raw(&'a [u8]),
}

enum PointerInfo<'a> {
    Block(&'a Block),
    Null,
    Invalid,
}

impl<'a> InstanceDataFormat<'a> {
    fn get(&self, start: usize, len: usize) -> &'a [u8] {
        match self {
            InstanceDataFormat::Block(block) => &block.data[start..start + len],
            InstanceDataFormat::Raw(data) => &data[start..start + len],
        }
    }

    fn code(&self) -> Option<[u8; 2]> {
        match self {
            InstanceDataFormat::Block(block) => Some([block.header.code[0], block.header.code[1]]),
            InstanceDataFormat::Raw(_) => None,
        }
    }

    fn old_memory_address(&self) -> Option<NonZeroU64> {
        match self {
            InstanceDataFormat::Block(block) => Some(block.header.old_memory_address),
            InstanceDataFormat::Raw(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct Instance<'a> {
    dna: &'a Dna,
    blend: &'a ParsedBlend,
    data: InstanceDataFormat<'a>,
    //We use a LinkedHashMap here because we want to preserve insertion order
    pub fields: LinkedHashMap<String, FieldTemplate>,
}

impl<'a> Instance<'a> {
    pub fn code(&self) -> [u8; 2] {
        self.data.code().expect("instance doesn't have a code")
    }

    fn expect_field(&self, name: &str) -> &FieldTemplate {
        match &self.fields.get(name) {
            Some(field) => field,
            None => panic!("invalid field '{}'", name),
        }
    }

    fn parse_ptr_address(&self, data: &[u8]) -> Option<NonZeroU64> {
        let address = match self.blend.header.pointer_size {
            PointerSize::Bits32 => parse_u32(data, self.blend.header.endianness) as u64,
            PointerSize::Bits64 => parse_u64(data, self.blend.header.endianness),
        };

        NonZeroU64::new(address)
    }

    /// Panics if field.info is not FieldInfo::Pointer
    fn get_ptr(&self, field: &FieldTemplate) -> PointerInfo<'a> {

        match field.info {
            FieldInfo::Pointer { .. } => {}
            _ => panic!(
                "get_ptr can only be called for pointer fields. ({:?})",
                field
            ),
        }

        let address = self.parse_ptr_address(&self.data.get(field.data_start, field.data_len));

        match address {
            None => PointerInfo::Null,
            Some(address) => {
                match self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                {
                    Some(block) => PointerInfo::Block(block),
                    None => PointerInfo::Invalid,
                }
            }
        }
    }

    pub fn is_valid<T: AsRef<str>>(&self, name: T) -> bool {
        let name = name.as_ref();
        let field = self.expect_field(name);

        match field.info {
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                assert_eq!(
                    field.data_len,
                    size_of::<u64>(),
                    "field '{}' doesn't have enough data for a pointer address. ({:?})",
                    name,
                    field
                );

                let pointer = self.get_ptr(field);

                match pointer {
                    PointerInfo::Null | PointerInfo::Invalid => false,
                    PointerInfo::Block(_) => true,
                }
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 2 => {
                let pointer = self.get_ptr(&field);

                let block = match pointer {
                    PointerInfo::Block(block) => block,
                    PointerInfo::Null | PointerInfo::Invalid => return false,
                };

                let pointer_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = block.data.len() / pointer_size;

                for i in 0..pointer_count {
                    let address = self.parse_ptr_address(&block.data[i * pointer_size..]);
                    //parse_u64(&block.data[i * ptr_size..], self.blend.header.endianness);

                    match address {
                        Some(address) => {
                            if !self
                                .blend
                                .blocks
                                .iter()
                                .any(|b| b.header.old_memory_address == address)
                            {
                                return false;
                            } else {
                                continue;
                            }
                        }
                        None => return false,
                    }
                }
                true
            }
            _ => panic!(
                "is_valid called for non-pointer field '{}'. ({:?})",
                name, field,
            ),
        }
    }

    fn get_value<T: AsRef<str>, U: BlendPrimitive>(&self, name: T) -> U {
        let name = name.as_ref();
        let field = self.expect_field(name);

        let blender_type_name = U::blender_name();

        match field.info {
            FieldInfo::Value if field.is_primitive && field.type_name == blender_type_name => {
                assert_eq!(
                    field.data_len,
                    size_of::<U>(),
                    "field '{}' doesn't have enough data for a {}. ({:?})",
                    name,
                    blender_type_name,
                    field,
                );

                U::parse(
                    &self.data.get(field.data_start, field.data_len),
                    self.blend.header.endianness,
                )
            }
            _ => panic!(
                "field '{}' is not {}. ({:?})",
                name, blender_type_name, field
            ),
        }
    }

    pub fn get_u8<T: AsRef<str>>(&self, name: T) -> u8 {
        self.get_value(name)
    }

    pub fn get_i8<T: AsRef<str>>(&self, name: T) -> i8 {
        self.get_value(name)
    }

    pub fn get_char<T: AsRef<str>>(&self, name: T) -> char {
        self.get_u8(name) as char
    }

    pub fn get_u16<T: AsRef<str>>(&self, name: T) -> u16 {
        self.get_value(name)
    }

    pub fn get_i16<T: AsRef<str>>(&self, name: T) -> i16 {
        self.get_value(name)
    }

    pub fn get_i32<T: AsRef<str>>(&self, name: T) -> i32 {
        self.get_value(name)
    }

    pub fn get_f32<T: AsRef<str>>(&self, name: T) -> f32 {
        self.get_value(name)
    }

    pub fn get_f64<T: AsRef<str>>(&self, name: T) -> f64 {
        self.get_value(name)
    }

    pub fn get_u64<T: AsRef<str>>(&self, name: T) -> u64 {
        self.get_value(name)
    }

    pub fn get_i64<T: AsRef<str>>(&self, name: T) -> i64 {
        self.get_value(name)
    }

    fn get_value_vec<T: AsRef<str>, U: BlendPrimitive>(&self, name: T) -> Vec<U> {
        let name = name.as_ref();
        let field = self.expect_field(name);
        let blender_type_name = U::blender_name();

        let data = match field.info {
            FieldInfo::ValueArray1D { len } if field.is_primitive => {
                assert_eq!(
                    field.data_len / len,
                    size_of::<U>(),
                    "field '{}' doesn't have enough data for a {} array. ({:?})",
                    name,
                    blender_type_name,
                    field,
                );

                self.data.get(field.data_start, field.data_len)
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                let pointer = self.get_ptr(&field);
                let block = match pointer {
                    PointerInfo::Block(block) => block,
                    PointerInfo::Null | PointerInfo::Invalid => panic!(
                        "field '{}' is a null or invalid pointer. ({:?})",
                        name, field
                    ),
                };

                let size = size_of::<U>();
                assert!(block.data.len() % size == 0);

                &block.data[..]
            }
            _ => panic!(
                "field '{}' is not a {} array. ({:?})",
                name, blender_type_name, field
            ),
        };

        data.chunks(size_of::<U>())
            .map(|s| U::parse(s, self.blend.header.endianness))
            .collect()
    }

    

    pub fn get_i32_vec<T: AsRef<str>>(&self, name: T) -> Vec<i32> {
        self.get_value_vec(name)
    }

    pub fn get_i16_vec<T: AsRef<str>>(&self, name: T) -> Vec<i16> {
        self.get_value_vec(name)
    }

    pub fn get_f32_vec<T: AsRef<str>>(&self, name: T) -> Vec<f32> {
        self.get_value_vec(name)
    }

    pub fn get_f64_vec<T: AsRef<str>>(&self, name: T) -> Vec<f64> {
        self.get_value_vec(name)
    }

    pub fn get_u64_vec<T: AsRef<str>>(&self, name: T) -> Vec<u64> {
        self.get_value_vec(name)
    }

    pub fn get_i64_vec<T: AsRef<str>>(&self, name: T) -> Vec<i64> {
        self.get_value_vec(name)
    }

    pub fn get_string<T: AsRef<str>>(&self, name: T) -> String {
        let name = name.as_ref();
        let field = self.expect_field(name);

        match field.info {
            FieldInfo::Value | FieldInfo::ValueArray1D { .. } => {
                if !field.is_primitive || field.type_name != "char" {
                    panic!(
                        "field '{}' is not a primitive or has the wrong type. ({:?})",
                        name, field
                    )
                }

                let data = &self.data.get(field.data_start, field.data_len);
                return data
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect();
            }
            _ => panic!("field '{}' is not a string. ({:?})", name, field),
        }
    }

    pub fn get<T: AsRef<str>>(&self, name: T) -> Instance<'a> {
        let name = name.as_ref();
        let field = self.expect_field(name);

        match field.info {
            FieldInfo::Value => {
                if field.is_primitive {
                    panic!("cannot access field '{}' as a struct. ({:?})", name, field,)
                }

                let r#struct = &self
                    .dna
                    .structs
                    .iter()
                    .find(|s| s.0 == field.type_index)
                    .unwrap_or_else(|| {
                        panic!(
                            "could not find type information for field '{}'. ({:?})",
                            name, field
                        )
                    });
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, self.dna, &self.blend.header);

                Instance {
                    dna: self.dna,
                    blend: self.blend,
                    data: InstanceDataFormat::Raw(self.data.get(field.data_start, field.data_len)),
                    fields,
                }
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                let pointer = self.get_ptr(&field);
                let block = match pointer {
                    PointerInfo::Block(block) => block,
                    PointerInfo::Null | PointerInfo::Invalid => panic!(
                        "field '{}' is null or doesn't point to a valid block. ({:?})",
                        name, field
                    ),
                };

                assert!(
                    block.header.count == 1,
                    "field '{}' is a list of structs, use get_instances to access. ({:?})",
                    name,
                    field
                );

                let fields = {
                    if &block.header.code != b"DATA" {
                        let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                        let r#type = &self.dna.types[r#struct.0 as usize];

                        generate_fields(r#struct, r#type, &self.dna, &self.blend.header)
                    } else {
                        if field.type_index >= 12 {
                            let r#struct = &self
                                .dna
                                .structs
                                .iter()
                                .find(|s| s.0 == field.type_index)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "could not find type information for field '{}'. ({:?})",
                                        name, field
                                    )
                                });
                            let r#type = &self.dna.types[r#struct.0 as usize];

                            generate_fields(r#struct, r#type, self.dna, &self.blend.header)
                        } else {
                            LinkedHashMap::new()
                        }
                    }
                };
                

                Instance {
                    dna: &self.dna,
                    blend: &self.blend,
                    data: InstanceDataFormat::Block(block),
                    fields,
                }
            }
            _ => panic!("field '{}' is not a valid struct ({:?})", name, field),
        }
    }

    //todo: return an actual vec here
    pub fn get_vec<T: AsRef<str>>(&self, name: T) -> impl Iterator<Item = Instance<'a>> {
        let name = name.as_ref();
        let field = self.expect_field(name);

        match field.info {
            FieldInfo::Value => {
                if field.type_name != "ListBase" {
                    panic!("field '{}' cannot be read as a list. ({:?})", name, field)
                }

                let list_instance = self.get(name);

                let last_address = list_instance
                    .get("last")
                    .data
                    .old_memory_address()
                    .expect("instance is not a root data block");
                let mut cur = list_instance.get("first");
                let mut instances = Vec::new();

                loop {
                    instances.push(cur.clone());

                    if cur
                        .data
                        .old_memory_address()
                        .expect("instance is not a root data block")
                        == last_address
                    {
                        break;
                    }

                    if !cur.is_valid("next") {
                        panic!(
                            "one of the elements of the field '{}' is invalid. ({:?})",
                            name, field
                        )
                    }

                    cur = cur.get("next");
                }

                //todo: stop hijacking the vector iterator implementation
                instances.into_iter()
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                let pointer = self.get_ptr(&field);
                let block = match pointer {
                    PointerInfo::Block(block) => block,
                    PointerInfo::Null | PointerInfo::Invalid => panic!(
                        "field '{}' is null or doesn't point to a valid block. ({:?})",
                        name, field
                    ),
                };

                //todo: check if sdna_index is valid (is block root or not?)

                let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                let mut instances = Vec::new();
                for i in 0..block.header.count as usize {
                    let data_len = (block.header.size / block.header.count) as usize;
                    let data_start = i * data_len;

                    instances.push(Instance {
                        dna: &self.dna,
                        blend: &self.blend,
                        data: InstanceDataFormat::Raw(
                            &block.data[data_start..data_start + data_len],
                        ),
                        fields: fields.clone(),
                    });
                }

                instances.into_iter()
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 2 => {
                let pointer = self.get_ptr(&field);
                let block = match pointer {
                    PointerInfo::Block(block) => block,
                    PointerInfo::Null | PointerInfo::Invalid => panic!(
                        "field '{}' is null or doesn't point to a valid block. ({:?})",
                        name, field
                    ),
                };

                let pointer_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = block.data.len() / pointer_size;

                let mut pointers = Vec::new();
                for i in 0..pointer_count {
                    let address = match self.parse_ptr_address(&block.data[i * pointer_size..]) {
                        None => panic!("field '{}' has a null element. '{:?}'", name, field),
                        Some(address) => address,
                    };

                    let block = self
                        .blend
                        .blocks
                        .iter()
                        .find(|b| b.header.old_memory_address == address);

                    match block {
                        Some(block) => {
                            let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                            let r#type = &self.dna.types[r#struct.0 as usize];

                            let fields =
                                generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                            pointers.push(Instance {
                                dna: &self.dna,
                                blend: &self.blend,
                                data: InstanceDataFormat::Block(block),
                                fields,
                            });
                        }
                        None => {
                            panic!("field '{}' has an invalid element. ({:?})", name, field);
                        }
                    }
                }

                pointers.into_iter()
            }
            _ => panic!(
                "field '{}' cannot be read as a list of structs ({:?})",
                name, field
            ),
        }
    }
}

pub struct Blend {
    /// `ParsedBlend` is an alias for the raw .blend file parsed by [blend_parse](todo:add_link).
    /// It contains the header and file-blocks of the .blend file.
    blend: ParsedBlend,
    dna: Dna,
}

impl Blend {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Blend {
        use std::fs::File;
        use std::io::Cursor;

        let mut file = File::open(path).expect("could not open .blend file");

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("could not read .blend file");

        Blend::new(Cursor::new(buffer))
    }

    pub fn new<T: Read>(data: T) -> Blend {
        let blend = ParsedBlend::from_data(data).unwrap();

        let dna = {
            let dna_block = &blend.blocks[blend.blocks.len() - 1];
            Dna::from_sdna_block(
                dna_block,
                blend.header.endianness,
                blend.header.pointer_size,
            )
            .unwrap()
        };

        Self { blend, dna }
    }

    pub fn get_all_root_blocks(&self) -> Vec<Instance> {
        self.blend
            .blocks
            .iter()
            .filter(|block| block.header.code[2..4] == [0, 0])
            .map(|block| {
                //
                assert!(
                    block.header.count == 1,
                    "blocks with a 2 letter code are assumed to not be lists"
                );

                let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                Instance {
                    dna: &self.dna,
                    blend: &self.blend,
                    data: InstanceDataFormat::Block(block),
                    fields,
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn get_by_code(&self, code: [u8; 2]) -> Vec<Instance> {
        self.blend
            .blocks
            .iter()
            .filter(|block| block.header.code[..2] == [code[0], code[1]])
            .map(|block| {
                //
                assert!(
                    block.header.count == 1,
                    "blocks with a 2 letter code are assumed to not be lists"
                );

                let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                Instance {
                    dna: &self.dna,
                    blend: &self.blend,
                    data: InstanceDataFormat::Block(block),
                    fields,
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn to_string(&self) -> String {
        
        enum InstanceNumber<'a> {
            Single(Instance<'a>),
            Many(Vec<Instance<'a>>),
        }

        enum InstanceToPrint<'a> {
            Root(Instance<'a>),
            FromField{ address: Option<NonZeroU64>, ident: usize, print_id: usize, field_template: FieldTemplate, instance: InstanceNumber<'a> },
        }

        let mut root_blocks = self.get_all_root_blocks();
        let mut seen_addresses : HashSet<_> = root_blocks.iter().map(|root_block| root_block.data.old_memory_address().expect("root blocks always have an old address")).collect();

        let mut instances_to_print: VecDeque<_> = 
            root_blocks            
            .into_iter()
            .map(|root_instance| InstanceToPrint::Root(root_instance))
            .collect();

        let mut final_string = String::new();
        let mut field_instance_print_id = 0_usize;

        fn field_to_string<'a>(
            field_name: &str, 
            field_template: &FieldTemplate, 
            instance: &Instance<'a>,
            ident: usize,
            field_instance_print_id: &mut usize,
            instances_to_print: &mut VecDeque<InstanceToPrint<'a>>,
            seen_addresses: &mut HashSet<NonZeroU64>,
            ) -> String {
            let ident_string: String = std::iter::repeat("    ").take(ident).collect();
            match field_template.info {
                FieldInfo::Value => {
                    let value_str = match &field_template.type_name[..] {
                        "int" => format!("{}", instance.get_i32(field_name)), 
                        "char" => format!("{}", instance.get_u8(field_name)), 
                        //"uchar" => format!("{}", instance.get_u8(field_name)), 
                        "short" => format!("{}", instance.get_i16(field_name)), 
                        //"ushort" => format!("{}", instance.get_u16(field_name)), 
                        "float" => format!("{}", instance.get_f32(field_name)), 
                        "double" => format!("{}", instance.get_f64(field_name)), 
                        //"long" => format!("{}", instance.get_i32(field_name)), 
                        //"ulong" => format!("{}", instance.get_i32(field_name)), 
                        "int64_t" => format!("{}", instance.get_i64(field_name)), 
                        "uint64_t" => format!("{}", instance.get_u64(field_name)),
                        name if field_template.is_primitive => panic!("unknown primitive {}", name),
                        _ => {
                            instances_to_print.push_back(InstanceToPrint::FromField{
                                address: None,
                                ident: ident + 1,
                                print_id: *field_instance_print_id,
                                field_template: field_template.clone(),
                                instance: InstanceNumber::Single(instance.get(field_name)),
                            });

                            *field_instance_print_id += 1;

                            format!("{{{}}}", *field_instance_print_id - 1)
                        }
                    };
                    
                    format!("{}    {}: {} = {}\n", ident_string, field_name, field_template.type_name, value_str.trim_right())
                }
                FieldInfo::Pointer { indirection_count: 1 } => {
                    let pointer = instance.get_ptr(field_template);

                    let value_str = match pointer {
                        PointerInfo::Invalid => String::from("invalid"),
                        PointerInfo::Null => String::from("null"),
                        PointerInfo::Block(block) => {
                            if seen_addresses.contains(&block.header.old_memory_address) {
                                format!("@{}", block.header.old_memory_address)
                            }
                            else {
                                if block.header.count == 1 {
                                    instances_to_print.push_back(InstanceToPrint::FromField {
                                        address: Some(block.header.old_memory_address),
                                        ident: ident + 1,
                                        print_id: *field_instance_print_id,
                                        field_template: field_template.clone(),
                                        instance: InstanceNumber::Single(instance.get(field_name)),
                                    });
                                } else {
                                    instances_to_print.push_back(InstanceToPrint::FromField {
                                        address: Some(block.header.old_memory_address),
                                        ident: ident + 1,
                                        print_id: *field_instance_print_id,
                                        field_template: field_template.clone(),
                                        instance: InstanceNumber::Many(instance.get_vec(field_name).collect()),
                                    });
                                }

                                seen_addresses.insert(block.header.old_memory_address);

                                *field_instance_print_id += 1;

                                format!("{{{}}}", *field_instance_print_id - 1)
                            }
                        }
                    };

                    format!("{}    {}: *{} = {}\n",
                        ident_string,
                        field_name,
                        field_template.type_name,
                        value_str,
                    )
                }
                _ => format!("{}    {}: {} = [xxx]\n", ident_string, field_name, field_template.type_name)
            }
        }

        while let Some(to_print) = instances_to_print.pop_front() {
            match to_print {
                InstanceToPrint::Root(instance) => {
                    match instance.data {
                        InstanceDataFormat::Block(block) => {
                            let (struct_type_index, _) = &self.dna.structs[block.header.sdna_index as usize];
                            let (instance_type_name, _) = &self.dna.types[*struct_type_index as usize];

                            let block_code = String::from_utf8_lossy(&block.header.code[0..2]);
                            final_string.push_str(
                                &format!("{} (code: {:?}) (address: {})\n", 
                                instance_type_name, 
                                block_code, 
                                block.header.old_memory_address
                            ));

                            for (field_name, field_template) in &instance.fields {
                                final_string.push_str(
                                    &field_to_string(
                                        field_name, 
                                        field_template, 
                                        &instance, 
                                        0,
                                        &mut field_instance_print_id,
                                        &mut instances_to_print,
                                        &mut seen_addresses,
                                    ));
                            }
                        },
                        InstanceDataFormat::Raw(_) => unreachable!("root blocks data is always InstanceDataFormat::Block")
                    }
                }
                InstanceToPrint::FromField{ address, ident, print_id, field_template, instance } => {
                    let mut field_string = if let Some(address) = address {
                        format!("{} (address: {})\n", field_template.type_name, address)
                    } else {
                        format!("{}\n", field_template.type_name)
                    };

                    match instance {
                        InstanceNumber::Single(instance) => {
                            for (field_name, field_template) in &instance.fields {
                                field_string.push_str(
                                    &field_to_string(
                                        field_name, 
                                        field_template, 
                                        &instance, 
                                        ident,
                                        &mut field_instance_print_id,
                                        &mut instances_to_print,
                                        &mut seen_addresses,
                                    ));
                            }
                        }
                        InstanceNumber::Many(ref instances) => {
                            let ident_string: String = std::iter::repeat("    ").take(ident).collect();
                            for instance in instances {
                                field_string.push_str(&format!("{}{{\n", ident_string));
                                for (field_name, field_template) in &instance.fields {
                                    field_string.push_str(
                                        &field_to_string(
                                            field_name, 
                                            field_template, 
                                            &instance, 
                                            ident,
                                            &mut field_instance_print_id,
                                            &mut instances_to_print,
                                            &mut seen_addresses,
                                        ));
                                }
                                field_string = field_string.trim_right().to_string();
                                field_string.push_str(&format!("{}\n{}and other {} elements ... \n{}}}\n", 
                                    ident_string, 
                                    ident_string, 
                                    instances.len() - 1, 
                                    ident_string
                                ));

                                break
                            }
                        }
                    }
                    
                    final_string = final_string.replacen(&format!("{{{}}}", print_id), &field_string.trim_right(), 1);
                }
            }
        }

        final_string
        /*let mut final_string = String::new();
        let mut root_blocks: VecDeque<(Option<usize>, Option<FieldTemplate>, Instance)> = 
            self.get_all_root_blocks()
            .into_iter()
            .map(|i| (None, None, i))
            .collect();

        while let Some((_, instance_template, instance)) = root_blocks.pop_front() {

            //TODO: vai ter que ser recursivo mesmo

            let (r#struct, r#type) : (&(u16, Vec<(u16, u16)>), &(String, u16))= {
                match (instance_template, &instance.data) {
                    (None, InstanceDataFormat::Block(block)) => {
                        let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                        let r#type = &self.dna.types[r#struct.0 as usize];

                        final_string.push_str(&format!("{} (@{})\n", r#type.0, block.header.old_memory_address));

                        (r#struct, r#type)
                    },
                    (Some(_), InstanceDataFormat::Block(_)) => { panic!("block instance with custom type");},
                    (Some(instance_template), InstanceDataFormat::Raw(_)) => {
                        let r#struct = &self
                            .dna
                            .structs
                            .iter()
                            .find(|s| s.0 == instance_template.type_index)
                            .expect("invalid type for struct");
                        let r#type = &self.dna.types[r#struct.0 as usize];

                        final_string.push_str(&format!("{}\n", r#type.0));

                        (r#struct, r#type)
                    }
                    (None, InstanceDataFormat::Raw(_)) => {
                        panic!("raw struct has no type information");
                    }
                }
            };

            for (field_name, field_template) in &instance.fields {
                final_string.push_str(&format!("    {}: {}", field_name, field_template.type_name));

                match field_template.info {
                    FieldInfo::Value => {
                        let value_str = match &field_template.type_name[..] {
                            "int" => format!("{}", instance.get_i32(field_name)), 
                            "char" => format!("{}", instance.get_u8(field_name)), 
                            //"uchar" => format!("{}", instance.get_u8(field_name)), 
                            "short" => format!("{}", instance.get_i16(field_name)), 
                            //"ushort" => format!("{}", instance.get_u16(field_name)), 
                            "float" => format!("{}", instance.get_f32(field_name)), 
                            "double" => format!("{}", instance.get_f64(field_name)), 
                            //"long" => format!("{}", instance.get_i32(field_name)), 
                            //"ulong" => format!("{}", instance.get_i32(field_name)), 
                            "int64_t" => format!("{}", instance.get_i64(field_name)), 
                            "uint64_t" => format!("{}", instance.get_u64(field_name)),
                            name if field_template.is_primitive => panic!("unknown primitive {}", name),
                            _ => {
                                let value_instance = instance.get(field_name);

                                root_blocks.push_back((Some(final_string.len()), Some(field_template.clone()), value_instance));

                                continue
                            }
                        };

                        final_string.push_str(&format!(" = {},\n", value_str));
                    }
                    FieldInfo::ValueArray1D { len } => {
                        let value_str = match &field_template.type_name[..] {
                            "int" => format!("{:?}", instance.get_i32_vec(field_name)), 
                            "char" => format!("{:?}", instance.get_string(field_name)), 
                            //"uchar" => format!("{}", instance.get_u8_vec(field_name)), 
                            "short" => format!("{:?}", instance.get_i16_vec(field_name)), 
                            //"ushort" => format!("{}", instance.get_u16_vec(field_name)), 
                            "float" => format!("{:?}", instance.get_f32_vec(field_name)), 
                            "double" => format!("{:?}", instance.get_f64_vec(field_name)), 
                            //"long" => format!("{}", instance.get_i32_vec(field_name)), 
                            //"ulong" => format!("{}", instance.get_i32_vec(field_name)), 
                            "int64_t" => format!("{:?}", instance.get_i64_vec(field_name)), 
                            "uint64_t" => format!("{:?}", instance.get_u64_vec(field_name)),
                            _ => String::new(),
                        };

                        final_string.push_str(&format!("[{}] = {},\n", len, value_str));
                    }
                    _ => final_string.push_str("\n")
                }
            }

            final_string.push_str("\n");
        }
        

        final_string*/
    }
}

fn generate_fields(
    r#struct: &(u16, Vec<(u16, u16)>),
    r#type: &(String, u16),
    dna: &Dna,
    header: &BlendHeader,
) -> LinkedHashMap<String, FieldTemplate> {
    let (_struct_type_index, struct_fields) = r#struct;
    let (_struct_type_name, struct_type_bytes_len) = r#type;

    let mut fields = LinkedHashMap::new();
    let mut data_start = 0;

    for (field_type_index, field_name_index) in struct_fields {
        let (field_type_name, field_type_bytes_len) = &dna.types[*field_type_index as usize];
        let field_full_name = &dna.names[*field_name_index as usize];

        let is_primitive = *field_type_index < 12;
        let (_, (field_name, field_info)) =
            parse_field(field_full_name).expect("field name could not be parsed");

        let field_bytes_len = match field_info {
            FieldInfo::Pointer { .. } | FieldInfo::FnPointer => header.pointer_size.bytes_num(),
            FieldInfo::PointerArray1D { len } => header.pointer_size.bytes_num() * len,
            FieldInfo::ValueArray1D { len } => *field_type_bytes_len as usize * len,
            FieldInfo::ValueArray2D { len1, len2 } => *field_type_bytes_len as usize * len1 * len2,
            FieldInfo::ValueArrayND { len1d, .. } => *field_type_bytes_len as usize * len1d,
            FieldInfo::Value => *field_type_bytes_len as usize,
        };

        fields.insert(
            String::from(field_name),
            FieldTemplate {
                info: field_info,
                type_index: *field_type_index,
                type_name: field_type_name.clone(),
                data_start,
                data_len: field_bytes_len,
                is_primitive,
            },
        );

        data_start += field_bytes_len;
    }
    assert_eq!(*struct_type_bytes_len as usize, data_start);

    fields
}

