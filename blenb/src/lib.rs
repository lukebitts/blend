mod field_parser;
mod primitive_parsers;
mod struct_parser;

use crate::field_parser::{parse_field, FieldInfo};
use crate::struct_parser::FieldTemplate;
use blend_parse::{Blend as ParsedBlend, Block, Header as BlendHeader, PointerSize};
use blend_sdna::Dna;
use linked_hash_map::LinkedHashMap;
use primitive_parsers::*;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

#[derive(Clone)]
pub enum InstanceDataFormat<'a> {
    Block(&'a Block),
    Raw(&'a [u8]),
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

    fn old_memory_address(&self) -> Option<u64> {
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

    //Should only be called for FieldTemplates which are pointers
    fn get_ptr(&self, field: &FieldTemplate) -> u64 {
        match self.blend.header.pointer_size {
            PointerSize::Bits32 => parse_u32(
                &self.data.get(field.data_start, field.data_len),
                self.blend.header.endianness,
            ) as u64,
            PointerSize::Bits64 => parse_u64(
                &self.data.get(field.data_start, field.data_len),
                self.blend.header.endianness,
            ),
        }
    }

    pub fn is_valid<T: AsRef<str>>(&self, name: T) -> bool {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                assert_eq!(
                    field.data_len,
                    size_of::<u64>(),
                    "field '{}' doesn't have enough data for a pointer address",
                    name
                );

                let address = self.get_ptr(field);

                if address == 0 {
                    false
                } else if !self
                    .blend
                    .blocks
                    .iter()
                    .any(|b| b.header.old_memory_address == address)
                {
                    false
                } else {
                    true
                }
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 2 => {
                let address = self.get_ptr(&field);
                let block = match self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                {
                    Some(block) => block,
                    None => return false,
                };

                let ptr_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = block.data.len() / ptr_size;

                for i in 0..pointer_count {
                    let address =
                        parse_u64(&block.data[i * ptr_size..], self.blend.header.endianness);

                    if address == 0 {
                        return false;
                    } else if !self
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
                true
            }
            _ => panic!(
                "Instance::is_valid should be used only for pointers, field '{}' is not a pointer",
                name
            ),
        }
    }

    pub fn get_f32<T: AsRef<str>>(&self, name: T) -> f32 {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value if field.is_primitive && field.type_name == "float" => {
                assert_eq!(
                    field.data_len,
                    size_of::<f32>(),
                    "field '{}' doesn't have enough data for a f32",
                    name
                );

                parse_f32(
                    &self.data.get(field.data_start, field.data_len),
                    self.blend.header.endianness,
                )
            }
            _ => panic!("field '{}' is not f32", name),
        }
    }

    pub fn get_f32_array<T: AsRef<str>>(&self, name: T) -> Vec<f32> {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::ValueArray1D { len } if field.is_primitive => {
                assert_eq!(
                    field.data_len / len,
                    size_of::<f32>(),
                    "field '{}' doesn't have enough data for a f32 array",
                    name
                );

                return self
                    .data
                    .get(field.data_start, field.data_len)
                    .chunks(size_of::<f32>())
                    .map(|data| parse_f32(data, self.blend.header.endianness))
                    .collect();
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                println!("{:?}", field);

                let address = self.get_ptr(&field);
                let block = self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                    .expect("invalid block address");

                let f32_size = size_of::<f32>();
                assert!(block.data.len() % f32_size == 0);

                block
                    .data
                    .chunks(f32_size)
                    .map(|s| parse_f32(s, self.blend.header.endianness))
                    .collect()
            }
            _ => panic!("field '{}' is not a f32 array ({:?})", name, field),
        }
    }

    pub fn get_i32<T: AsRef<str>>(&self, name: T) -> i32 {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value if field.is_primitive && field.type_name == "int" => {
                assert_eq!(
                    field.data_len,
                    size_of::<i32>(),
                    "field '{}' doesn't have enough data for a i32",
                    name
                );

                parse_i32(
                    &self.data.get(field.data_start, field.data_len),
                    self.blend.header.endianness,
                )
            }
            _ => panic!("field '{}' is not i32", name),
        }
    }

    pub fn get_i16_array<T: AsRef<str>>(&self, name: T) -> Vec<i16> {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::ValueArray1D { len } if field.is_primitive => {
                assert_eq!(
                    field.data_len / len,
                    size_of::<i16>(),
                    "field '{}' doesn't have enough data for a i16 array",
                    name
                );

                return self
                    .data
                    .get(field.data_start, field.data_len)
                    .chunks(size_of::<i16>())
                    .map(|data| parse_i16(data, self.blend.header.endianness))
                    .collect();
            }
            _ => panic!("field '{}' is not a i16 array"),
        }
    }

    pub fn get_string<T: AsRef<str>>(&self, name: T) -> String {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value | FieldInfo::ValueArray1D { .. } => {
                if !field.is_primitive || field.type_name != "char" {
                    panic!("field '{}' is not a primitive or has the wrong type", name)
                }

                let data = &self.data.get(field.data_start, field.data_len);
                return data
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect();
            }
            _ => panic!("field '{}' is not a string", name),
        }
    }

    pub fn get_instance<T: AsRef<str>>(&self, name: T) -> Instance<'a> {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value => {
                if field.is_primitive {
                    panic!(
                        "cannot access field '{}' as a struct, as it is a primitive",
                        name
                    )
                }

                let r#struct = &self
                    .dna
                    .structs
                    .iter()
                    .find(|s| s.0 == field.type_index)
                    .unwrap_or_else(|| {
                        panic!("could not find type information for field '{}'", name)
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
                if !self.is_valid(name) {
                    panic!("field '{}' is null or doesn't point to a valid block", name);
                }

                let address = self.get_ptr(&field);
                let block = self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                    .expect("invalid block address");

                assert!(
                    block.header.count == 1,
                    "field '{}' is a list of structs, use get_instances to access",
                    name
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
            }
            _ => panic!("field '{}' is not a valid struct ({:?})", name, field),
        }
    }

    pub fn get_instances<T: AsRef<str>>(&self, name: T) -> impl Iterator<Item = Instance<'a>> {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value => {
                if field.type_name != "ListBase" {
                    panic!("")
                }

                let list_instance = self.get_instance(name);

                let last = list_instance.get_instance("last");
                let mut cur = list_instance.get_instance("first");
                let mut instances = Vec::new();

                loop {
                    instances.push(cur.clone());

                    if cur.data.old_memory_address().unwrap()
                        == last.data.old_memory_address().unwrap()
                    {
                        break;
                    }

                    cur = cur.get_instance("next");
                }

                //todo: stop hijacking the vector iterator implementation
                instances.into_iter()
            }
            FieldInfo::Pointer { indirection_count } if indirection_count == 1 => {
                let address = self.get_ptr(&field);
                let block = self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                    .expect("invalid block address");

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
                let address = self.get_ptr(&field);
                let block = self
                    .blend
                    .blocks
                    .iter()
                    .find(|b| b.header.old_memory_address == address)
                    .expect("invalid block address");

                let ptr_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = block.data.len() / ptr_size;

                let mut pointers = Vec::new();
                for i in 0..pointer_count {
                    let address =
                        parse_u64(&block.data[i * ptr_size..], self.blend.header.endianness);

                    if address == 0 {
                        panic!(
                            "null pointer exception on get_instances, field {} '{:?}'",
                            name, field
                        );
                    }

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
                            panic!(
                                "invalid pointers on get_instances, field {} '{:?}'",
                                name, field
                            );
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

    pub fn get_by_code(&self, code: [u8; 2]) -> impl Iterator<Item = Instance> {
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
            .into_iter()
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

