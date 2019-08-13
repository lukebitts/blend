use crate::parsers::{
    blend::{Blend as ParsedBlend, Block, Header as BlendHeader},
    dna::{Dna, DnaStruct, DnaType},
    field::{parse_field, FieldInfo},
    primitive::*,
    PointerSize,
};
use linked_hash_map::LinkedHashMap;
use std::{io::Read, mem::size_of, num::NonZeroU64, path::Path};

/// An `Instance`'s data can be a reference to a `Block` if the `Instance` represents a root or subsidiary block,
/// or it can be raw bytes if the `Instance` was created by accessing a field in another `Instance`.
#[derive(Clone)]
pub enum InstanceDataFormat<'a> {
    Block(&'a Block),
    Raw(&'a [u8]),
}

/// Pointers in the blend file are valid if the file contains another block with the correct address. They are invalid 
/// if no block is found with the correct address.
pub enum PointerInfo<'a> {
    Block(&'a Block),
    Null,
    Invalid,
}

impl<'a> InstanceDataFormat<'a> {
    /// `get` simplifies the access to the underlying data inside the `InstanceDataFormat`.
    fn get(&self, start: usize, len: usize) -> &'a [u8] {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                    &data.data[start..start + len]
                }
                _ => unimplemented!(),
            },
            InstanceDataFormat::Raw(data) => &data[start..start + len],
        }
    }

    /// Returns the code of the underlying block, if it has one. 
    /// # Panics
    /// Panics if called on subsidiary blocks
    fn code(&self) -> Option<[u8; 2]> {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { code, .. } => Some([code[0], code[1]]),
                Block::Subsidiary { .. } => unimplemented!("no code for subsidiary"),
                _ => unimplemented!(),
            },
            InstanceDataFormat::Raw(_) => None,
        }
    }

    /// Returns the memory address of the underlying block, if it has one.
    pub fn memory_address(&self) -> Option<NonZeroU64> {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { memory_address, .. }
                | Block::Subsidiary { memory_address, .. } => Some(*memory_address),
                _ => unimplemented!(),
            },
            InstanceDataFormat::Raw(_) => None,
        }
    }
}

/// Represents a field inside a struct. The data `FieldTemplate` keeps is used to interpret the raw bytes of the block.
#[derive(Debug, Clone)]
pub struct FieldTemplate {
    //pub name: String,
    pub info: FieldInfo,
    /// The index of this field's type inside the `Dna::types` array.
    pub type_index: usize,
    /// The type name of this field. Used for pretty printing and some sanity checks.
    pub type_name: String,
    /// The index of the data in the `Instance` owned by this field.
    pub data_start: usize,
    /// The length in bytes of the data in the `Instance` owned by this field.
    pub data_len: usize,
    /// A field can represent a primitive or a struct.
    pub is_primitive: bool,
}

/// Represents a block of data inside the blend file. An `Instance` can be a camera, a mesh, a material, or anything
/// else Blender uses internally, like material nodes, user settings or render options. An `Instance` is conceptually a 
/// `struct`: a collection of named fields which can themselves be structs or primitives.
#[derive(Clone)]
pub struct Instance<'a> {
    /// References to the `Dna` and the `ParsedBlend` are kept because we only interpret data when the user accesses it.
    dna: &'a Dna,
    blend: &'a ParsedBlend,
    /// The raw binary data this `Instance` owns.
    pub data: InstanceDataFormat<'a>,
    /// The fields of this `Instance`. 
    pub fields: LinkedHashMap<String, FieldTemplate>, //We use a LinkedHashMap here because we want to preserve insertion order
}

impl<'a> std::fmt::Debug for Instance<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Instance {{ fields: {:?} }}", self.fields)
    }
}

impl<'a> Instance<'a> {
    /// If this `Instance` was created from a primary/root `Block` it will have a code. Possible codes include "OB" for
    /// objects, "ME" for meshes, "CA" for cameras, etc.
    /// # Panics
    /// Panics if the instance underlying data doesn't have a code
    pub fn code(&self) -> [u8; 2] {
        self.data.code().expect("instance doesn't have a code")
    }

    /// If this `Instance` was created from a primary/root or subsidiary `Block` it will have a memory address. Blender
    /// dumps its memory into the blend file when saving and the old memory addresses are used the recreate the 
    /// connections between blocks when loading the file again.
    /// /// # Panics
    /// Panics if the instance underlying data doesn't have an old memory address.
    pub fn memory_address(&self) -> NonZeroU64 {
        self.data
            .memory_address()
            .expect("instance doesn't have memory address")
    }

    /// `expect_field` simplifies accessing a field since most of the time panicking is the correct response for an
    /// invalid field name.
    fn expect_field(&self, name: &str) -> &FieldTemplate {
        match &self.fields.get(name) {
            Some(field) => field,
            None => panic!("invalid field '{}'", name),
        }
    }

    fn parse_ptr_address(&self, data: &[u8]) -> Option<NonZeroU64> {
        let address = match self.blend.header.pointer_size {
            PointerSize::Bits32 => u64::from(parse_u32(data, self.blend.header.endianness)),
            PointerSize::Bits64 => parse_u64(data, self.blend.header.endianness),
        };

        NonZeroU64::new(address)
    }

    /// Used internally to get a block behind a pointer, use `Instance::get` or `Instance::get_vec`
    /// instead of this unless you know what you are doing. Check `Blend::to_string` for proper usage.
    /// # Panics
    /// Panics if field.info is not FieldInfo::Pointer
    /// 
    /// Panics if the block pointed by this field is not Block::Principal or Block::Subsidiary
    pub fn get_ptr(&self, field: &FieldTemplate) -> PointerInfo<'a> {
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
                match self.blend.blocks.iter().find(|b| match b {
                    Block::Principal { memory_address, .. }
                    | Block::Subsidiary { memory_address, .. } => *memory_address == address,
                    _ => false,
                }) {
                    Some(block) => PointerInfo::Block(block),
                    None => PointerInfo::Invalid,
                }
            }
        }
    }

    /// Tests whether a pointer is valid or not.
    /// # Panics
    /// Panics if field `name` is not a pointer.
    pub fn is_valid<T: AsRef<str>>(&self, name: T) -> bool {
        //println!("is valid? {}", name.as_ref());
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
                let pointer_count = match block {
                    Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                        data.data.len() / pointer_size
                    }
                    _ => unimplemented!(),
                };

                for i in 0..pointer_count {
                    match block {
                        Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                            let address = self.parse_ptr_address(&data.data[i * pointer_size..]);
                            //parse_u64(&block.data[i * ptr_size..], self.blend.header.endianness);

                            match address {
                                Some(address) => {
                                    if !self.blend.blocks.iter().any(|b| match b {
                                        Block::Principal { memory_address, .. }
                                        | Block::Subsidiary { memory_address, .. } => {
                                            *memory_address == address
                                        }
                                        _ => false,
                                    }) {
                                        return false;
                                    } else {
                                        continue;
                                    }
                                }
                                None => return false,
                            }
                        }
                        _ => unimplemented!(),
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

    /// `get_value` abstracts accessing primitives and is used by all `get_[]` functions (`get_i8`, `get_f32`, etc).
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

    /// `get_value_vec` abstracts accessing primitive arrays and is used by all `get_[]_vec` functions (`get_i8_vec`, 
    /// `get_f32_vec`, etc).
    fn get_value_vec<T: AsRef<str>, U: BlendPrimitive>(&self, name: T) -> Vec<U> {
        let name = name.as_ref();
        let field = self.expect_field(name);
        let blender_type_name = U::blender_name();

        let data = match field.info {
            FieldInfo::ValueArray { len, .. } if field.is_primitive => {
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
                match block {
                    Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                        assert!(data.data.len() % size == 0);

                        &data.data[..]
                    }
                    _ => unimplemented!(),
                }
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
            FieldInfo::Value | FieldInfo::ValueArray { .. } => {
                if !field.is_primitive || field.type_name != "char" {
                    panic!(
                        "field '{}' is not a primitive or has the wrong type. ({:?})",
                        name, field
                    )
                }

                let data = &self.data.get(field.data_start, field.data_len);
                data.iter()
                    .take_while(|c| **c != 0)
                    .map(|c| *c as u8 as char)
                    .collect()
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
                    .find(|s| s.type_index == field.type_index)
                    .unwrap_or_else(|| {
                        panic!(
                            "could not find type information for field '{}'. ({:?})",
                            name, field
                        )
                    });
                let r#type = &self.dna.types[r#struct.type_index];

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

                let fields = match block {
                    Block::Principal {
                        data, dna_index, ..
                    } => {
                        assert!(
                            data.count == 1,
                            "field '{}' is a list of structs, use get_instances to access. ({:?})",
                            name,
                            field
                        );

                        //todo: rename to dna_struct etc
                        let r#struct = &self.dna.structs[*dna_index];
                        let r#type = &self.dna.types[r#struct.type_index];

                        generate_fields(r#struct, r#type, &self.dna, &self.blend.header)
                    }
                    Block::Subsidiary {
                        data, dna_index, ..
                    } => {
                        assert!(
                            data.count == 1,
                            "field '{}' is a list of structs, use get_instances to access. ({:?})",
                            name,
                            field
                        );

                        if field.type_index >= 12 {
                            if *dna_index >= 12 {
                                //assert_eq!(field.type_index as u16, self.dna.structs[block.header.sdna_index as usize].0);
                                let r#struct = &self.dna.structs[*dna_index];
                                let r#type = &self.dna.types[r#struct.type_index];
                                generate_fields(r#struct, r#type, self.dna, &self.blend.header)
                            } else if let Some(r#struct) = &self
                                .dna
                                .structs
                                .iter()
                                .find(|s| s.type_index == field.type_index)
                            {
                                let r#type = &self.dna.types[r#struct.type_index as usize];
                                generate_fields(r#struct, r#type, self.dna, &self.blend.header)
                            } else {
                                unreachable!("impossible type")
                            }
                        } else {
                            let r#struct = &self.dna.structs[*dna_index];
                            if r#struct.type_index >= 12 {
                                let r#type = &self.dna.types[r#struct.type_index];
                                generate_fields(r#struct, r#type, &self.dna, &self.blend.header)
                            } else {
                                unreachable!("impossible type")
                            }
                        }
                    }
                    _ => unimplemented!(),
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

    //todo: return a vec here?
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
                    .memory_address()
                    .expect("instance is not a root data block");
                let mut cur = list_instance.get("first");
                let mut instances = Vec::new();

                loop {
                    instances.push(cur.clone());

                    if cur
                        .data
                        .memory_address()
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

                //todo: return a custom iterator 
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

                match block {
                    Block::Principal {
                        data, dna_index, ..
                    } => {
                        let r#struct = &self.dna.structs[*dna_index];
                        let r#type = &self.dna.types[r#struct.type_index];

                        let fields =
                            generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                        let mut instances = Vec::new();
                        for i in 0..data.count as usize {
                            let data_len = (data.data.len() / data.count) as usize;
                            let data_start = i * data_len;

                            instances.push(Instance {
                                dna: &self.dna,
                                blend: &self.blend,
                                data: InstanceDataFormat::Raw(
                                    &data.data[data_start..data_start + data_len],
                                ),
                                fields: fields.clone(),
                            });
                        }

                        instances.into_iter()
                    }
                    Block::Subsidiary {
                        data, dna_index, ..
                    } => {
                        let fields = if field.type_index >= 12 {
                            if *dna_index >= 12 {
                                //assert_eq!(field.type_index as u16, self.dna.structs[block.header.sdna_index as usize].0);
                                let r#struct = &self.dna.structs[*dna_index];
                                let r#type = &self.dna.types[r#struct.type_index];
                                generate_fields(r#struct, r#type, self.dna, &self.blend.header)
                            } else if let Some(r#struct) = &self
                                .dna
                                .structs
                                .iter()
                                .find(|s| s.type_index == field.type_index)
                            {
                                let r#type = &self.dna.types[r#struct.type_index as usize];
                                generate_fields(r#struct, r#type, self.dna, &self.blend.header)
                            } else {
                                unreachable!("impossible type")
                            }
                        } else {
                            let r#struct = &self.dna.structs[*dna_index];
                            if r#struct.type_index >= 12 {
                                let r#type = &self.dna.types[r#struct.type_index];
                                generate_fields(r#struct, r#type, &self.dna, &self.blend.header)
                            } else {
                                unreachable!("impossible type")
                            }
                        };

                        let mut instances = Vec::new();
                        for i in 0..data.count as usize {
                            let data_len = (data.data.len() / data.count) as usize;
                            let data_start = i * data_len;

                            instances.push(Instance {
                                dna: &self.dna,
                                blend: &self.blend,
                                data: InstanceDataFormat::Raw(
                                    &data.data[data_start..data_start + data_len],
                                ),
                                fields: fields.clone(),
                            });
                        }

                        instances.into_iter()
                    }
                    _ => unimplemented!(),
                }
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
                let pointer_count = match block {
                    Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                        data.data.len() / pointer_size
                    }
                    _ => unimplemented!(),
                };

                let mut pointers = Vec::new();
                for i in 0..pointer_count {
                    let address = match block {
                        Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                            match self.parse_ptr_address(&data.data[i * pointer_size..]) {
                                None => {
                                    panic!("field '{}' has a null element. '{:?}'", name, field)
                                }
                                Some(address) => address,
                            }
                        }
                        _ => unimplemented!(),
                    };

                    let block = self.blend.blocks.iter().find(|b| match b {
                        Block::Principal { memory_address, .. }
                        | Block::Subsidiary { memory_address, .. } => *memory_address == address,
                        _ => false //unimplemented!("{:?} {:?}", b, field),
                    });

                    match block {
                        Some(Block::Principal { dna_index, .. }) => {
                            let r#struct = &self.dna.structs[*dna_index];
                            let r#type = &self.dna.types[r#struct.type_index];

                            let fields =
                                generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                            pointers.push(Instance {
                                dna: &self.dna,
                                blend: &self.blend,
                                data: InstanceDataFormat::Block(
                                    block.expect("we are sure block is some here"),
                                ),
                                fields,
                            });
                        }
                        Some(_) => unimplemented!(),
                        None => {
                            continue;
                            //panic!("field '{}' has an invalid element. ({:?})", name, field);
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
    /// `blend` field contains the header, file-blocks and dna of the .blend file, which are used in runtime to 
    /// interpret the blend file data.
    pub blend: ParsedBlend,
}

impl Blend {
    pub fn from_path<T: AsRef<Path>>(path: T) -> Blend {
        use std::{fs::File, io::Cursor};

        let mut file = File::open(path).expect("could not open .blend file");

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("could not read .blend file");

        Blend::new(Cursor::new(buffer))
    }

    pub fn new<T: Read>(data: T) -> Blend {
        let blend = ParsedBlend::from_data(data).unwrap();

        Self { blend }
    }

    pub fn get_all_root_blocks(&self) -> Vec<Instance> {
        self.blend
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Principal { dna_index, .. } => {
                    let dna_struct = &self.blend.dna.structs[*dna_index];
                    let dna_type = &self.blend.dna.types[dna_struct.type_index];

                    let fields =
                        generate_fields(dna_struct, dna_type, &self.blend.dna, &self.blend.header);

                    Some(Instance {
                        dna: &self.blend.dna,
                        blend: &self.blend,
                        data: InstanceDataFormat::Block(block),
                        fields,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn get_by_code(&self, search_code: [u8; 2]) -> Vec<Instance> {
        self.blend
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Principal {
                    data,
                    dna_index,
                    code,
                    ..
                } if *code == search_code => {
                    assert!(
                        data.count == 1,
                        "blocks with a 2 letter code are assumed to not be lists"
                    );

                    let r#struct = &self.blend.dna.structs[*dna_index];
                    let r#type = &self.blend.dna.types[r#struct.type_index];

                    let fields =
                        generate_fields(r#struct, r#type, &self.blend.dna, &self.blend.header);

                    Some(Instance {
                        dna: &self.blend.dna,
                        blend: &self.blend,
                        data: InstanceDataFormat::Block(block),
                        fields,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    }
}

fn generate_fields(
    dna_struct: &DnaStruct,
    dna_type: &DnaType,
    dna: &Dna,
    header: &BlendHeader,
) -> LinkedHashMap<String, FieldTemplate> {
    let mut fields = LinkedHashMap::new();
    let mut data_start = 0;

    for field in &dna_struct.fields {
        let field_dna_type = &dna.types[field.type_index];
        let field_full_name = &dna.names[field.name_index];

        let is_primitive = field.type_index < 12;
        let (_, (field_name, field_info)) =
            parse_field(field_full_name).expect("field name could not be parsed");

        let field_bytes_len = match field_info {
            FieldInfo::Pointer { .. } | FieldInfo::FnPointer => header.pointer_size.bytes_num(),
            FieldInfo::PointerArray { len, .. } => header.pointer_size.bytes_num() * len,
            FieldInfo::ValueArray { len, .. } => field_dna_type.bytes_len * len,
            FieldInfo::Value => field_dna_type.bytes_len,
        };

        fields.insert(
            String::from(field_name),
            FieldTemplate {
                info: field_info,
                type_index: field.type_index,
                type_name: field_dna_type.name.clone(),
                data_start,
                data_len: field_bytes_len,
                is_primitive,
            },
        );

        data_start += field_bytes_len;
    }
    assert_eq!(dna_type.bytes_len, data_start);

    fields
}
