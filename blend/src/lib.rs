#![feature(never_type, concat_idents)]

extern crate blend_parse;
extern crate blend_sdna;
extern crate byteorder;
extern crate num;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate nom;
extern crate linked_hash_map;

mod field_parser;
mod primitive_parsers;

use blend_parse::{Blend as ParsedBlend, Block, Endianness};
use blend_sdna::Dna;
use field_parser::{parse_field, FieldInfo};
use primitive_parsers::{
    parse_f32, parse_f64, parse_i16, parse_i32, parse_i64, parse_i8, parse_u16, parse_u64, parse_u8,
};
//use std::collections::HashMap;
use linked_hash_map::LinkedHashMap as HashMap;
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub struct FieldTemplate {
    pub name: String,
    pub info: FieldInfo,
    pub type_index: u16,
    pub type_name: String,
    pub data_start: usize,
    pub data_len: usize,
    pub is_primitive: bool,
}

impl FieldTemplate {
    pub fn is_single_value(&self) -> bool {
        match self.info {
            FieldInfo::Value => true,
            _ => false,
        }
    }

    pub fn is_value_or_value_array(&self) -> bool {
        match self.info {
            FieldInfo::Value | FieldInfo::ValueArray1D { .. } | FieldInfo::ValueArray2D { .. } => {
                true
            }
            _ => false,
        }
    }

    pub fn is_pointer(&self) -> bool {
        match self.info {
            FieldInfo::Pointer { .. } | FieldInfo::PointerArray1D { .. } | FieldInfo::FnPointer => {
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum BlendPrimitive {
    Int(i32),
    IntArray1D(Vec<i32>),
    IntArray2D(Vec<Vec<i32>>),
    Char(i8),
    CharArray1D(Vec<i8>),
    CharArray2D(Vec<Vec<i8>>),
    UChar(u8),
    UCharArray1D(Vec<u8>),
    UCharArray2D(Vec<Vec<u8>>),
    Short(i16),
    ShortArray1D(Vec<i16>),
    ShortArray2D(Vec<Vec<i16>>),
    UShort(u16),
    UShortArray1D(Vec<u16>),
    UShortArray2D(Vec<Vec<u16>>),
    Float(f32),
    FloatArray1D(Vec<f32>),
    FloatArray2D(Vec<Vec<f32>>),
    Double(f64),
    DoubleArray1D(Vec<f64>),
    DoubleArray2D(Vec<Vec<f64>>),
    //Long(!),
    //LongArray1D(!),
    //LongArray2D(!),
    //ULong(!),
    //ULongArray1D(!),
    //ULongArray2D(!),
    Int64(i64),
    Int64Array1D(Vec<i64>),
    Int64Array2D(Vec<Vec<i64>>),
    UInt64(u64),
    UInt64Array1D(Vec<u64>),
    UInt64Array2D(Vec<Vec<u64>>),
    Void,
}

macro_rules! field_convert (
    (
        $template:ident,
        $field_data:expr,
        $endianness:expr,
        ($(
            ($str_name:expr,
            $f_type:ty,
            $prim_type:path,
            $prim_type_array1d:path,
            $prim_type_array2d:path,
            $converter:path)
        ),*)
    ) => {
        match (&$template.info, &$template.type_name[..]) {
            $(
                (&FieldInfo::Value, $str_name) => {
                    assert_eq!($field_data.len(), std::mem::size_of::<$f_type>());
                    $prim_type($converter($field_data, $endianness))
                }
                (&FieldInfo::ValueArray1D { len }, $str_name) => {
                    assert_eq!($field_data.len() / len, std::mem::size_of::<$f_type>());
                    $prim_type_array1d(
                    $field_data
                        .chunks($field_data.len() / len)
                        .map(|data| $converter(data, $endianness))
                        .collect(),
                    )
                }
                (&FieldInfo::ValueArray2D { len1, len2 }, $str_name) => {
                    assert_eq!($field_data.len() / (len1 * len2), std::mem::size_of::<$f_type>());
                    $prim_type_array2d(
                    $field_data
                        .chunks($field_data.len() / len1)
                        .map(|data| {
                            data
                                .chunks(data.len() / len2)
                                .map(|data| $converter(data, $endianness))
                                .collect()
                        })
                        .collect(),
                    )
                }
            )*
            _ => panic!("invalid conversion"),
        }
    }
);

impl BlendPrimitive {
    pub fn from_template(template: &FieldTemplate, endianness: Endianness, block: &Block) -> Self {
        if !template.is_primitive || template.is_pointer() {
            panic!("can't create primitive from non-primtive and/or pointer template");
        }

        let field_data = &block.data[template.data_start..template.data_start + template.data_len];

        //primitive types: int, char, uchar, short, ushort, float, double, long, ulong, int64_t, uint64_t

        field_convert!(
            template,
            field_data,
            endianness,
            (
                (
                    "int",
                    i32,
                    BlendPrimitive::Int,
                    BlendPrimitive::IntArray1D,
                    BlendPrimitive::IntArray2D,
                    parse_i32
                ),
                (
                    "char",
                    i8,
                    BlendPrimitive::Char,
                    BlendPrimitive::CharArray1D,
                    BlendPrimitive::CharArray2D,
                    parse_i8
                ),
                (
                    "uchar",
                    u8,
                    BlendPrimitive::UChar,
                    BlendPrimitive::UCharArray1D,
                    BlendPrimitive::UCharArray2D,
                    parse_u8
                ),
                (
                    "short",
                    i16,
                    BlendPrimitive::Short,
                    BlendPrimitive::ShortArray1D,
                    BlendPrimitive::ShortArray2D,
                    parse_i16
                ),
                (
                    "ushort",
                    u16,
                    BlendPrimitive::UShort,
                    BlendPrimitive::UShortArray1D,
                    BlendPrimitive::UShortArray2D,
                    parse_u16
                ),
                (
                    "float",
                    f32,
                    BlendPrimitive::Float,
                    BlendPrimitive::FloatArray1D,
                    BlendPrimitive::FloatArray2D,
                    parse_f32
                ),
                (
                    "double",
                    f64,
                    BlendPrimitive::Double,
                    BlendPrimitive::DoubleArray1D,
                    BlendPrimitive::DoubleArray2D,
                    parse_f64
                ),
                (
                    "int64_t",
                    i64,
                    BlendPrimitive::Int64,
                    BlendPrimitive::Int64Array1D,
                    BlendPrimitive::Int64Array2D,
                    parse_i64
                ),
                (
                    "uint64_t",
                    u64,
                    BlendPrimitive::UInt64,
                    BlendPrimitive::UInt64Array1D,
                    BlendPrimitive::UInt64Array2D,
                    parse_u64
                )
            )
        )
    }
}

#[derive(Debug)]
pub enum FieldInstance {
    Value(BlendPrimitive),
    Struct(StructInstance),
}

#[derive(Debug)]
pub struct StructInstance {
    type_name: String,
    fields: HashMap<String, FieldInstance>,
}

impl StructInstance {
    pub fn to_string(self) -> String {
        let mut ret = format!("{} {{\n", self.type_name);

        for (field_name, field_instance) in &self.fields {
            ret.push_str(&format!("\t{} = {:?}\n", field_name, field_instance)[..]);
        }

        ret.push_str("}\n");

        ret
    }
}

pub fn main() {
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
            let (field_type_name, field_type_bytes_len) = &dna.types[*field_type_index as usize];
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

            fields.push(FieldTemplate {
                name: String::from(field_name),
                info: field_info,
                type_index: *field_type_index,
                type_name: field_type_name.clone(),
                data_start,
                data_len: field_bytes_len,
                is_primitive,
            });

            data_start += field_bytes_len;
        }
        assert_eq!(*struct_type_bytes_len as usize, data_start);
        templates.insert(*struct_type_index, fields);
    }

    let mut instance_structs = Vec::new();

    for block in &blend.blocks {
        if block.header.code[2..=3] == [0, 0] {
            let (struct_type_index, _) = &dna.structs[block.header.sdna_index as usize];
            let struct_template = &templates[struct_type_index];
            let (struct_type_name, _) = &dna.types[*struct_type_index as usize];

            let mut instance_fields: HashMap<String, FieldInstance> = HashMap::new();

            for field in struct_template.iter() {
                if field.is_primitive && field.is_value_or_value_array() {
                    instance_fields.insert(
                        field.name.clone(),
                        FieldInstance::Value(BlendPrimitive::from_template(
                            field,
                            blend.header.endianness,
                            block,
                        )),
                    );
                }
            }

            instance_structs.push(StructInstance {
                type_name: struct_type_name.clone(),
                fields: instance_fields,
            });
        }
    }

    for s in instance_structs {
        println!("{}", s.to_string());
    }
}
