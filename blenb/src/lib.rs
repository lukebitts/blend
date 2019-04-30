mod field_parser;
mod primitive_parsers;
mod struct_parser;

use crate::field_parser::{parse_field, FieldInfo};
use crate::struct_parser::FieldTemplate;
use blend_parse::{Blend as ParsedBlend, Block, Header as BlendHeader};
use blend_sdna::Dna;
use linked_hash_map::LinkedHashMap;
use primitive_parsers::*;
use std::io::Read;
use std::mem::size_of;
use std::path::Path;

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
}

pub struct Instance<'a> {
    dna: &'a Dna,
    blend_header: &'a BlendHeader,
    data: InstanceDataFormat<'a>,
    //We use a LinkedHashMap here because we want to preserve insertion order
    pub fields: LinkedHashMap<String, FieldTemplate>,
}

impl<'a> Instance<'a> {
    pub fn get_f32<T: AsRef<str>>(&self, name: T) -> f32 {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        match field.info {
            FieldInfo::Value if field.type_name == "float" => {
                assert_eq!(
                    field.data_len,
                    size_of::<f32>(),
                    "field '{}' doesn't have enough data for a f32",
                    name
                );

                parse_f32(
                    &self.data.get(field.data_start, field.data_len),
                    self.blend_header.endianness,
                )
            }
            _ => panic!("field '{}' is not f32", name),
        }
    }

    pub fn get_instance<T: AsRef<str>>(&self, name: T) -> Instance<'a> {
        let name = name.as_ref();
        let field = &self
            .fields
            .get(name)
            .unwrap_or_else(|| panic!("invalid field '{}'", name));

        if field.is_primitive {
            panic!("cannot access field '{}' as a struct, as it is a primitive")
        }

        match field.info {
            FieldInfo::Value => {
                //
                let r#struct = &self
                    .dna
                    .structs
                    .iter()
                    .find(|s| s.0 == field.type_index)
                    .unwrap_or_else(|| {
                        panic!("could not find type information for field '{}'", name)
                    });
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, self.dna, self.blend_header);

                Instance {
                    dna: self.dna,
                    blend_header: self.blend_header,
                    data: InstanceDataFormat::Raw(self.data.get(field.data_start, field.data_len)),
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

        vec![].into_iter()
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
        use std::io::{Cursor, Read};

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
            .filter(|block| block.header.code[..2] == code[..])
            .map(|block| {
                //
                assert!(block.header.count == 1);

                let r#struct = &self.dna.structs[block.header.sdna_index as usize];
                let r#type = &self.dna.types[r#struct.0 as usize];

                let fields = generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                Instance {
                    dna: &self.dna,
                    blend_header: &self.blend.header,
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
    let (struct_type_index, struct_fields) = r#struct;
    let (_struct_type_name, struct_type_bytes_len) = r#type;

    println!("{:?} -- {:?}", r#struct, r#type);

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
