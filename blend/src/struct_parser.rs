use blend_parse::{Blend as ParsedBlend, Block, Endianness, PointerSize};
use blend_sdna::Dna;
use field_parser::FieldInfo;
use linked_hash_map::LinkedHashMap as HashMap;
use primitive_parsers::{
    parse_f32, parse_f64, parse_i16, parse_i32, parse_i64, parse_i8, parse_u16, parse_u32,
    parse_u64, parse_u8,
};
use std::rc::Rc;

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

#[derive(Clone)]
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

impl ::std::fmt::Debug for BlendPrimitive {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use self::BlendPrimitive::*;

        match self {
            Int(v) => write!(f, "Int({:?})", v),
            IntArray1D(v) => write!(f, "IntArray1D({:?})", v),
            IntArray2D(v) => write!(f, "IntArray2D({:?})", v),
            Char(v) => write!(f, "Char({:?})", v),
            CharArray1D(v) => {
                let data: String = v
                    .iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect();
                write!(f, "CharArray1D(\"{}\")", data)
            }
            CharArray2D(v) => write!(f, "CharArray2D({:?})", v),
            UChar(v) => write!(f, "UChar({:?})", v),
            UCharArray1D(v) => write!(f, "UCharArray1D({:?})", v),
            UCharArray2D(v) => write!(f, "UCharArray2D({:?})", v),
            Short(v) => write!(f, "Short({:?})", v),
            ShortArray1D(v) => write!(f, "ShortArray1D({:?})", v),
            ShortArray2D(v) => write!(f, "ShortArray2D({:?})", v),
            UShort(v) => write!(f, "UShort({:?})", v),
            UShortArray1D(v) => write!(f, "UShortArray1D({:?})", v),
            UShortArray2D(v) => write!(f, "UShortArray2D({:?})", v),
            Float(v) => write!(f, "Float({:?})", v),
            FloatArray1D(v) => write!(f, "FloatArray1D({:?})", v),
            FloatArray2D(v) => write!(f, "FloatArray2D({:?})", v),
            Double(v) => write!(f, "Double({:?})", v),
            DoubleArray1D(v) => write!(f, "DoubleArray1D({:?})", v),
            DoubleArray2D(v) => write!(f, "DoubleArray2D({:?})", v),
            Int64(v) => write!(f, "Int64({:?})", v),
            Int64Array1D(v) => write!(f, "Int64Array1D({:?})", v),
            Int64Array2D(v) => write!(f, "Int64Array2D({:?})", v),
            UInt64(v) => write!(f, "UInt64({:?})", v),
            UInt64Array1D(v) => write!(f, "UInt64Array1D({:?})", v),
            UInt64Array2D(v) => write!(f, "UInt64Array2D({:?})", v),
            Void => write!(f, "Void"),
        }
    }
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
                    assert_eq!($field_data.len(), ::std::mem::size_of::<$f_type>());
                    $prim_type($converter($field_data, $endianness))
                }
                (&FieldInfo::ValueArray1D { len }, $str_name) => {
                    assert_eq!($field_data.len() / len, ::std::mem::size_of::<$f_type>());
                    $prim_type_array1d(
                    $field_data
                        .chunks($field_data.len() / len)
                        .map(|data| $converter(data, $endianness))
                        .collect(),
                    )
                }
                (&FieldInfo::ValueArray2D { len1, len2 }, $str_name) => {
                    assert_eq!($field_data.len() / (len1 * len2), ::std::mem::size_of::<$f_type>());
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
    pub fn from_template(template: &FieldTemplate, endianness: Endianness, data: &[u8]) -> Self {
        if !template.is_primitive || template.is_pointer() {
            panic!("can't create primitive from non-primtive and/or pointer template");
        }

        let field_data = &data[template.data_start..template.data_start + template.data_len];

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PointerInfo {
    Invalid,
    Null,
    Address(u64, FieldInfo),
    PointerToStruct,
    PointerToFunction,
    PointerToPointer,
}

impl PointerInfo {
    pub fn from_template(
        template: &FieldTemplate,
        endianness: Endianness,
        pointer_size: PointerSize,
        data: &[u8],
        all_blocks: &[Block],
    ) -> Self {
        if !template.is_pointer() {
            panic!("can't create pointer info from non-pointer template");
        }

        let field_data = &data[template.data_start..template.data_start + template.data_len];

        let count = if let FieldInfo::PointerArray1D { len } = template.info {
            len
        } else {
            1
        };
        assert_eq!(field_data.len(), pointer_size.bytes_num() * count);

        let addr = match pointer_size {
            PointerSize::Bits32 => parse_u32(field_data, endianness) as u64,
            PointerSize::Bits64 => parse_u64(field_data, endianness),
        };

        match template.info {
            FieldInfo::FnPointer => PointerInfo::PointerToFunction,
            _ => {
                if addr == 0 {
                    PointerInfo::Null
                } else if all_blocks
                    .iter()
                    .filter(|b| b.header.old_memory_address == addr)
                    .next()
                    .is_none()
                {
                    PointerInfo::Invalid
                } else {
                    PointerInfo::Address(addr, template.info.clone())
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldInstance {
    Value(BlendPrimitive),
    Struct(StructInstanceData),
    Pointer(PointerInfo),
    PointerList(Vec<PointerInfo>),
}

#[derive(Debug, Clone)]
pub struct StructInstanceData {
    pub type_name: String,
    pub fields: HashMap<String, FieldInstance>,
}

#[derive(Debug, Clone)]
pub enum StructData {
    Single(StructInstanceData),
    List(Vec<StructInstanceData>),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct StructInstance {
    pub type_name: String,
    pub code: Option<[u8; 2]>,
    pub old_memory_address: Option<u64>,
    pub data: StructData,
}

impl StructInstance {
    pub fn to_string(&self, tab_count: usize) -> String {
        let tabs = ::std::iter::repeat("\t")
            .take(tab_count)
            .collect::<String>();

        let print_single = |this: &StructInstanceData, tab_count: usize, show_addr: bool| {
            let tabs = ::std::iter::repeat("\t")
                .take(tab_count)
                .collect::<String>();

            let mut ret = if let Some(addr) = self.old_memory_address {
                if show_addr {
                    format!("{} ({}) {{\n", self.type_name, addr)
                } else {
                    format!("{{\n")
                }
            } else {
                format!("{} {{\n", self.type_name)
            };

            for (field_name, field_instance) in &this.fields {
                match field_instance {
                    FieldInstance::Struct(instance) => {
                        ret.push_str(
                            &format!(
                                "{}\t{} = {}\n",
                                tabs,
                                field_name,
                                StructInstance {
                                    type_name: instance.type_name.clone(),
                                    code: None,
                                    old_memory_address: None,
                                    data: StructData::Single(instance.clone()),
                                }
                                .to_string(tab_count + 1),
                            )[..],
                        );
                    }
                    _ => ret.push_str(
                        &format!("{}\t{} = {:?}\n", tabs, field_name, field_instance)[..],
                    ),
                }
            }

            ret.push_str(&format!("{}}}", tabs)[..]);

            ret
        };

        match &self.data {
            StructData::Single(instance) => print_single(&instance, tab_count, true),
            StructData::List(instances) => {
                let mut ret = String::new();
                ret.push_str(&format!(
                    "{}{} ({}) [",
                    tabs,
                    self.type_name,
                    self.old_memory_address.unwrap()
                ));
                for instance in instances {
                    ret.push_str(&format!("\n\t{}", tabs));
                    ret.push_str(&print_single(&instance, tab_count + 1, false));
                    //break;
                }
                ret.push_str(&format!("\n{}]", tabs));
                ret
            }
            StructData::Raw(data) => {
                let mut ret = String::new();

                ret.push_str(&format!(
                    "{} ({}) {{\n",
                    self.type_name,
                    self.old_memory_address.unwrap()
                ));
                ret.push_str(&format!("{}\tdata = {:?}\n", tabs, data));
                ret.push_str(&format!("{}}}", tabs)[..]);

                ret
            }
        }
    }
}

pub fn data_to_struct(
    instance_structs: &mut HashMap<u64, Rc<StructInstance>>,
    seen_addresses: &mut ::std::collections::HashSet<u64>,
    templates: &HashMap<u16, Vec<FieldTemplate>>,
    struct_template: &Vec<FieldTemplate>,
    struct_type_index: usize,
    blend: &ParsedBlend,
    dna: &Dna,
    data: &[u8],
) -> StructInstanceData {
    let (struct_type_name, _) = &dna.types[struct_type_index as usize];

    let mut instance_fields: HashMap<String, FieldInstance> = HashMap::new();

    'field: for field in struct_template.iter() {
        if field.is_primitive && field.is_value_or_value_array() {
            instance_fields.insert(
                field.name.clone(),
                FieldInstance::Value(BlendPrimitive::from_template(
                    field,
                    blend.header.endianness,
                    data,
                )),
            );
        } else if !field.is_primitive && field.is_value_or_value_array() {
            let struct_template = &templates[&field.type_index];
            let struct_type_index = field.type_index;

            instance_fields.insert(
                field.name.clone(),
                FieldInstance::Struct(data_to_struct(
                    instance_structs,
                    seen_addresses,
                    templates,
                    struct_template,
                    struct_type_index as usize,
                    blend,
                    dna,
                    &data[field.data_start..field.data_start + field.data_len],
                )),
            );
        } else if field.is_pointer() {
            let info = PointerInfo::from_template(
                field,
                blend.header.endianness,
                blend.header.pointer_size,
                data,
                &blend.blocks[..],
            );

            match &info {
                PointerInfo::Invalid | PointerInfo::Null => {
                    instance_fields.insert(field.name.clone(), FieldInstance::Pointer(info));
                }
                PointerInfo::PointerToFunction => {
                    instance_fields.insert(field.name.clone(), FieldInstance::Pointer(info));
                }
                PointerInfo::PointerToStruct | PointerInfo::PointerToPointer => {
                    panic!("no pointer should have this type yet")
                }
                PointerInfo::Address(addr, field_info) => {
                    if seen_addresses.contains(addr) {
                        instance_fields.insert(field.name.clone(), FieldInstance::Pointer(info));
                        continue;
                    }
                    seen_addresses.insert(*addr);

                    match field_info {
                        FieldInfo::Pointer {
                            indirection_count: 1,
                        } => {
                            let block = blend
                                .blocks
                                .iter()
                                .filter(|b| b.header.old_memory_address == *addr)
                                .next()
                                .unwrap();

                            if block.header.code[2..=3] == [0, 0] {
                                instance_fields
                                    .insert(field.name.clone(), FieldInstance::Pointer(info));
                                continue;
                            }

                            let (struct_type_index, struct_template) = if field.type_index < 12 {
                                if block.header.sdna_index == 0 {
                                    // We don't have enough type information to parse this block. As far as I
                                    // understand this means this is a primitive array. We don't know which
                                    // primitive type though, since the field.type_index doesn't carry
                                    // this information, being always set to 0. The user has to decide the type
                                    // when accessing this.

                                    instance_structs.insert(
                                        *addr,
                                        Rc::new(StructInstance {
                                            type_name: String::from("[Unknown Type]"),
                                            code: Some([
                                                block.header.code[0],
                                                block.header.code[1],
                                            ]),
                                            old_memory_address: Some(
                                                block.header.old_memory_address,
                                            ),
                                            data: StructData::Raw(block.data.clone()),
                                        }),
                                    );

                                    instance_fields
                                        .insert(field.name.clone(), FieldInstance::Pointer(info));
                                    continue 'field;
                                }
                                let (struct_type_index, _) =
                                    &dna.structs[block.header.sdna_index as usize];
                                (*struct_type_index, &templates[struct_type_index])
                            } else {
                                if let Some(template) = templates.get(&field.type_index) {
                                    (field.type_index, template)
                                } else {
                                    let (struct_type_index, _) =
                                        &dna.structs[block.header.sdna_index as usize];
                                    (field.type_index, &templates[&struct_type_index])
                                }
                            };

                            let instance = block_to_struct(
                                instance_structs,
                                seen_addresses,
                                templates,
                                Some(block.header.old_memory_address),
                                Some([block.header.code[0], block.header.code[1]]),
                                struct_template,
                                struct_type_index as usize,
                                blend,
                                dna,
                                block,
                            );

                            instance_structs.insert(*addr, Rc::new(instance));
                            instance_fields
                                .insert(field.name.clone(), FieldInstance::Pointer(info));
                        }
                        FieldInfo::Pointer {
                            indirection_count: 2,
                        } => {
                            let block = blend
                                .blocks
                                .iter()
                                .filter(|b| b.header.old_memory_address == *addr)
                                .next()
                                .unwrap();

                            let ptr_size = blend.header.pointer_size.bytes_num();
                            let pointer_count = block.data.len() / ptr_size;

                            let mut pointers = Vec::new();
                            for i in 0..pointer_count {
                                let addr =
                                    parse_u64(&block.data[i * ptr_size..], blend.header.endianness);

                                if addr == 0 {
                                    pointers.push(PointerInfo::Null);
                                    continue;
                                }

                                let block_exists = blend
                                    .blocks
                                    .iter()
                                    .filter(|b| b.header.old_memory_address == addr)
                                    .next()
                                    .is_some();

                                if !block_exists {
                                    pointers.push(PointerInfo::Invalid);
                                    continue;
                                }

                                pointers.push(PointerInfo::Address(
                                    addr,
                                    FieldInfo::Pointer {
                                        indirection_count: 1,
                                    },
                                ));
                            }

                            instance_fields
                                .insert(field.name.clone(), FieldInstance::PointerList(pointers));
                        }
                        _ => {
                            instance_fields
                                .insert(field.name.clone(), FieldInstance::Pointer(info));
                        }
                    }
                }
            }
        }
    }

    StructInstanceData {
        type_name: struct_type_name.clone(),
        fields: instance_fields,
    }
}

pub fn block_to_struct(
    instance_structs: &mut HashMap<u64, Rc<StructInstance>>,
    seen_addresses: &mut ::std::collections::HashSet<u64>,
    templates: &HashMap<u16, Vec<FieldTemplate>>,
    old_memory_address: Option<u64>,
    code: Option<[u8; 2]>,
    struct_template: &Vec<FieldTemplate>,
    struct_type_index: usize,
    blend: &ParsedBlend,
    dna: &Dna,
    block: &Block,
) -> StructInstance {
    if block.header.count == 1 {
        StructInstance {
            type_name: dna.types[struct_type_index].0.clone(),
            code,
            old_memory_address,
            data: StructData::Single(data_to_struct(
                instance_structs,
                seen_addresses,
                templates,
                struct_template,
                struct_type_index,
                blend,
                dna,
                &block.data[..],
            )),
        }
    } else {
        let mut instances = Vec::new();
        let type_len = struct_template
            .iter()
            .fold(0, |accum, field| accum + field.data_len);
        for i in 0..block.header.count as usize {
            instances.push(data_to_struct(
                instance_structs,
                seen_addresses,
                templates,
                struct_template,
                struct_type_index,
                blend,
                dna,
                &block.data[(i * type_len)..(i + 1) * type_len],
            ));
        }
        StructInstance {
            type_name: dna.types[struct_type_index].0.clone(),
            code,
            old_memory_address,
            data: StructData::List(instances),
        }
    }
}
