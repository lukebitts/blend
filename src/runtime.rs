use crate::parsers::{
    blend::{Block, BlockData, Header as BlendHeader, RawBlend},
    dna::{Dna, DnaStruct, DnaType},
    field::{parse_field, FieldInfo},
    primitive::*,
    Endianness, PointerSize,
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
    /// `get` accesses only a specifc slice of the underlying data.
    pub fn get(&self, start: usize, len: usize) -> &'a [u8] {
        &self.data()[start..start + len]
    }

    /// Simplifies the access to the underlying data inside the `InstanceDataFormat`.
    pub fn data(&self) -> &'a [u8] {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { data, .. }
                | Block::Subsidiary { data, .. }
                | Block::Global { data, .. } => &data.data[..],
                _ => unimplemented!(),
            },
            InstanceDataFormat::Raw(data) => &data[..],
        }
    }

    /// Returns the code of the underlying block, if it has one.
    fn code(&self) -> Option<[u8; 4]> {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { code, .. } => Some([code[0], code[1], 0, 0]),
                Block::Global { .. } => Some(*b"GLOB"),
                Block::Rend { .. } => Some(*b"REND"),
                Block::Test { .. } => Some(*b"TEST"),
                Block::Dna { .. } => Some(*b"DNA1"),
                Block::Subsidiary { .. } => None,
            },
            InstanceDataFormat::Raw(_) => None,
        }
    }

    /// Returns the memory address of the underlying block, if it has one.
    pub fn memory_address(&self) -> Option<NonZeroU64> {
        match self {
            InstanceDataFormat::Block(block) => match block {
                Block::Principal { memory_address, .. }
                | Block::Subsidiary { memory_address, .. }
                | Block::Global { memory_address, .. } => Some(*memory_address),
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
    blend: &'a RawBlend,
    pub type_name: String,
    /// The raw binary data this `Instance` owns.
    pub data: InstanceDataFormat<'a>,
    /// The fields of this `Instance`.
    pub fields: LinkedHashMap<String, FieldTemplate>, //We use a LinkedHashMap here because we want to preserve insertion order
}

impl<'a> std::fmt::Debug for Instance<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance")
            .field("type_name", &self.type_name)
            .field("fields", &self.fields)
            .finish()
    }
}

//todo fix
use std::fmt;

#[allow(clippy::cognitive_complexity)]
fn fmt_instance(
    seen_addresses: &mut std::collections::HashSet<NonZeroU64>,
    f: &mut fmt::Formatter,
    inst: &Instance,
    ident: usize,
) -> fmt::Result {
    let ident_str: String = std::iter::repeat(" ").take(4 * ident).collect();

    write!(f, "{}", inst.type_name)?;

    match (inst.data.code(), inst.data.memory_address()) {
        (Some(code), Some(memory_address)) => {
            write!(
                f,
                " (code:{}|@{})",
                String::from_utf8_lossy(&code[0..=1]),
                memory_address
            )?;
        }
        (Some(code), None) => {
            write!(f, " (code:{})", String::from_utf8_lossy(&code[0..=1]))?;
        }
        (None, Some(memory_address)) => {
            write!(f, " (@{})", memory_address)?;
        }
        (None, None) => {}
    }

    writeln!(f, " {{")?;

    for (field_name, field) in inst.fields.iter().filter(|(n, _)| !n.starts_with("_pad")) {
        match &field.info {
            FieldInfo::Value => {
                write!(f, "{}    {}: ", ident_str, field_name)?;
                match &field.type_name[..] {
                    "int" => writeln!(f, "{} = {};", field.type_name, inst.get_i32(field_name))?,
                    "char" => writeln!(f, "{} = {};", field.type_name, inst.get_u8(field_name))?,
                    "short" => writeln!(f, "{} = {};", field.type_name, inst.get_i16(field_name))?,
                    "float" => writeln!(f, "{} = {};", field.type_name, inst.get_f32(field_name))?,
                    "double" => writeln!(f, "{} = {};", field.type_name, inst.get_f64(field_name))?,
                    "int64_t" => {
                        writeln!(f, "{} = {};", field.type_name, inst.get_i64(field_name))?
                    }
                    "uint64_t" => {
                        writeln!(f, "{} = {};", field.type_name, inst.get_u64(field_name))?
                    }
                    _ if field.is_primitive => panic!("unknown primitive"),
                    _ => {
                        if field.type_name == "ListBase" {
                            if inst.is_valid(field_name) {
                                let list_base_instance = inst.get_iter(field_name).next().unwrap();
                                writeln!(
                                    f,
                                    "ListBase<{}>[#?] = [",
                                    list_base_instance.type_name,
                                    //list_base_instances.len()
                                )?;
                                if list_base_instance.data.code().is_none() {
                                    for i in Some(list_base_instance) {
                                        if !seen_addresses.contains(&i.memory_address()) {
                                            seen_addresses.insert(i.memory_address());
                                            write!(f, "{}        ", ident_str)?;
                                            fmt_instance(seen_addresses, f, &i, ident + 2)?;
                                        } else {
                                            writeln!(
                                                f,
                                                "{}        @{},",
                                                ident_str,
                                                i.memory_address()
                                            )?;
                                        }
                                        break;
                                    }
                                } else {
                                    unimplemented!()
                                    /*write!(
                                        f,
                                        "{}",
                                        list_base_instances
                                            .iter()
                                            .map(|i| format!("{}", i.memory_address()))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )?;*/
                                }
                                writeln!(f, "{}    ];", ident_str)?;
                            } else {
                                writeln!(f, "ListBase<?>[] = null;")?;
                            }
                        } else {
                            fmt_instance(seen_addresses, f, &inst.get(field_name), ident + 1)?;
                        }
                    }
                }
            }
            FieldInfo::ValueArray { dimensions, .. } => {
                write!(
                    f,
                    "{}    {}: {}{:?} = ",
                    ident_str, field_name, field.type_name, dimensions
                )?;
                match &field.type_name[..] {
                    "char" => {
                        let data = inst.data.get(field.data_start, field.data_len);

                        // Some char arrays might be interpreted as strings if their first element is 0.
                        if let Ok(string_data) = String::from_utf8(
                            data.iter().take_while(|&&b| b != 0).cloned().collect(),
                        ) {
                            writeln!(f, "\"{}\";", string_data)?;
                        } else {
                            writeln!(f, "{:?};", inst.get_u8_vec(field_name))?;
                        }
                    }
                    "int" => writeln!(f, "{:?};", inst.get_i32_vec(field_name))?,
                    "short" => writeln!(f, "{:?};", inst.get_i16_vec(field_name))?,
                    "float" => writeln!(f, "{:?};", inst.get_f32_vec(field_name))?,
                    "double" => writeln!(f, "{:?};", inst.get_f64_vec(field_name))?,
                    "int64_t" => writeln!(f, "{:?};", inst.get_i64_vec(field_name))?,
                    "uint64_t" => writeln!(f, "{:?};", inst.get_u64_vec(field_name))?,
                    _ if field.is_primitive => panic!("unknown primitive"),
                    _ => {
                        writeln!(f, "[")?;
                        let instances = inst.get_iter(field_name);
                        for i in instances {
                            write!(f, "{}        ", ident_str)?;
                            fmt_instance(seen_addresses, f, &i, ident + 2)?;
                            break;
                        }
                        writeln!(f, "{}    ];", ident_str)?;
                    }
                }
            }
            FieldInfo::Pointer {
                indirection_count: 1,
            } => {
                if ["next", "prev", "first", "last"]
                    .iter()
                    .any(|n| n == field_name)
                {
                    if inst.is_valid(field_name) {
                        writeln!(
                            f,
                            "{}    {}: {} = (@{});",
                            ident_str,
                            field_name,
                            inst.get(field_name).type_name,
                            inst.parse_ptr_address(
                                &inst.data.get(field.data_start, field.data_len)
                            )
                            .unwrap()
                        )?
                    } else {
                        writeln!(
                            f,
                            "{}    {}: {} = null;",
                            ident_str, field_name, field.type_name
                        )?
                    }
                //} else if inst.is_valid(field_name) {
                } else if inst.is_valid(field_name) {
                    //let ptr_field = &inst.fields[field_name];
                    let ptr_inst = inst.get(field_name);
                    //assert!(!seen_addresses.contains(&ptr_inst.memory_address()));
                    if ptr_inst.data.code().is_none()
                        && !seen_addresses.contains(&inst.get(field_name).memory_address())
                    {
                        if ptr_inst.type_name == "Link" {
                            writeln!(
                                f,
                                "{}    {}: {}* = (not enough type information);",
                                ident_str, field_name, field.type_name
                            )?
                        } else {
                            seen_addresses.insert(ptr_inst.memory_address());
                            match ptr_inst.data {
                                InstanceDataFormat::Block(block) => match block {
                                    Block::Principal { data, .. }
                                    | Block::Subsidiary { data, .. } => {
                                        if data.count > 1 {
                                            writeln!(
                                                f,
                                                "{}    {}: {}[{}] = [",
                                                ident_str, field_name, field.type_name, data.count
                                            )?;
                                            for p in inst.get_iter(field_name) {
                                                //write!(f, "{}    {}: {} = ", ident_str, field_name, field.type_name)?;
                                                write!(f, "{}        ", ident_str)?;
                                                fmt_instance(seen_addresses, f, &p, ident + 2)?;
                                                break;
                                            }
                                            writeln!(f, "{}    ];", ident_str)?;
                                        } else {
                                            write!(
                                                f,
                                                "{}    {}: {} = ",
                                                ident_str, field_name, field.type_name
                                            )?;
                                            fmt_instance(seen_addresses, f, &ptr_inst, ident + 1)?;
                                        }
                                    }
                                    _ => unimplemented!(),
                                },
                                _ => unimplemented!(),
                            }
                        }
                    } else {
                        writeln!(
                            f,
                            "{}    {}: {} = (@{});",
                            ident_str,
                            field_name,
                            field.type_name,
                            inst.parse_ptr_address(
                                &inst.data.get(field.data_start, field.data_len)
                            )
                            .unwrap()
                        )?
                    }
                } else {
                    writeln!(
                        f,
                        "{}    {}: {} = null;",
                        ident_str, field_name, field.type_name
                    )?;
                }
            }
            FieldInfo::Pointer {
                indirection_count: 2,
            } => {
                if inst.is_valid(field_name) {
                    let mut instances = inst.get_iter(field_name);
                    write!(
                        f,
                        "{}    {}: {}[%?] = [",
                        ident_str,
                        field_name,
                        field.type_name,
                        //instances.len(),
                    )?;
                    if let Some(instance) = instances.next() {
                        write!(f, "{}", instance)?;
                    }
                    writeln!(f, "];")?;
                } else {
                    writeln!(
                        f,
                        "{}    {}: {}[] = null;",
                        ident_str, field_name, field.type_name
                    )?;
                }
            }
            FieldInfo::FnPointer => writeln!(f, "{}    {}: fn() = null", ident_str, field_name)?,
            FieldInfo::PointerArray { dimensions, .. } => {
                let mut instances = inst.get_iter(field_name);
                writeln!(
                    f,
                    "{}    {}: {}{:?}!? = $ [",
                    ident_str, field_name, field.type_name, dimensions,
                )?;

                if let Some(instance) = instances.next() {
                    if instance.data.code().is_none()
                        && !seen_addresses.contains(&instance.memory_address())
                    {
                        seen_addresses.insert(instance.memory_address());
                        write!(f, "{}        ", ident_str)?;
                        if instance.type_name == "Link" {
                            writeln!(f, "(not enough type information);")?
                        } else {
                            fmt_instance(seen_addresses, f, &instance, ident + 2)?;
                        }
                    } else {
                        write!(f, "{}        @{}", ident_str, instance.memory_address())?;
                    }
                }
                writeln!(f, "{}    ];", ident_str)?;
            }
            _ => unimplemented!("unknown type"),
        }
    }

    writeln!(f, "{}}}", ident_str)
}

impl fmt::Display for Instance<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_instance(&mut std::collections::HashSet::new(), f, &self, 0)
    }
}

/*#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Validness {
    Valid,
    InvalidType,
    Invalid,
}

impl Into<bool> for Validness {
    fn into(self) -> bool {
        match self {
            Validness::Valid => true,
            Validness::InvalidType => false,
            Validness::Invalid => false,
        }
    }
}*/

fn parse_ptr_address(
    data: &[u8],
    pointer_size: PointerSize,
    endianness: Endianness,
) -> Option<NonZeroU64> {
    let address = match pointer_size {
        PointerSize::Bits32 => u64::from(parse_u32(data, endianness)),
        PointerSize::Bits64 => parse_u64(data, endianness),
    };

    NonZeroU64::new(address)
}

impl<'a> Instance<'a> {
    /// If this `Instance` was created from a primary/root `Block` it will have a code. Possible codes include "OB" for
    /// objects, "ME" for meshes, "CA" for cameras, etc.
    /// # Panics
    /// Panics if the instance's underlying data doesn't have a code
    pub fn code(&self) -> [u8; 4] {
        self.data.code().expect("instance doesn't have a code")
    }

    /// If this `Instance` was created from a primary/root or subsidiary `Block` it will have a memory address. Blender
    /// dumps its memory into the blend file when saving and the old memory addresses are used the recreate the
    /// connections between blocks when loading the file again.
    /// # Panics
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
        parse_ptr_address(
            data,
            self.blend.header.pointer_size,
            self.blend.header.endianness,
        )
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

    /// Tests whether a field is valid and can be accessed using the `get` methods without panicking.
    pub fn is_valid<T: AsRef<str>>(&self, name: T) -> bool {
        let name = name.as_ref();

        if !self.fields.contains_key(name) {
            return false;
        }

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
                    PointerInfo::Block(block) => match block {
                        Block::Principal { .. } | Block::Subsidiary { .. } => true,
                        _ => unimplemented!(),
                    },
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
            FieldInfo::FnPointer => false,
            FieldInfo::PointerArray { .. } => unimplemented!(), //todo: fix
            FieldInfo::Value => {
                if field.type_name == "ListBase" {
                    let instance = self.get(name);
                    instance.is_valid("first") && instance.is_valid("last")
                } else {
                    true
                }
            }
            FieldInfo::ValueArray { .. } => true,
            _ => panic!(
                "is_valid called for unknown field '{}'. ({:?})",
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

    pub fn get_u8_vec<T: AsRef<str>>(&self, name: T) -> Vec<u8> {
        self.get_value_vec(name)
    }

    pub fn get_i8_vec<T: AsRef<str>>(&self, name: T) -> Vec<i8> {
        self.get_value_vec(name)
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
                    type_name: r#type.name.clone(),
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

                let (fields, r#type) = match block {
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

                        (
                            generate_fields(r#struct, r#type, &self.dna, &self.blend.header),
                            r#type,
                        )
                    }
                    Block::Subsidiary { dna_index, .. } => {
                        if let Some(v) = generate_subsidiary_fields(
                            &self.dna,
                            &self.blend.header,
                            field,
                            *dna_index,
                        ) {
                            v
                        } else {
                            println!("{:#?}", self);
                            println!("{}: {:?}\n{:?}", name, block, field);
                            panic!()
                        }
                    }
                    _ => unimplemented!(),
                };
                Instance {
                    dna: &self.dna,
                    blend: &self.blend,
                    type_name: r#type.name.clone(),
                    data: InstanceDataFormat::Block(block),
                    fields,
                }
            }
            _ => panic!("field '{}' is not a valid struct ({:?})", name, field),
        }
    }

    //todo: return a vec here?
    #[allow(clippy::cognitive_complexity)]
    pub fn get_vec<T: AsRef<str>>(&self, name: T) -> impl Iterator<Item = Instance<'a>> {
        let name = name.as_ref();
        let field = self.expect_field(name);

        match field.info {
            FieldInfo::Value => {
                if field.type_name != "ListBase" {
                    panic!("field '{}' cannot be read as a list. ({:?})", name, field)
                }

                let list_instance = self.get(name);

                let last_address = list_instance.get("last").memory_address();
                let mut cur = list_instance.get("first");
                let mut instances = Vec::new();

                loop {
                    instances.push(cur.clone());

                    if cur.memory_address() == last_address {
                        break;
                    }

                    while !cur.is_valid("next") {
                        cur = cur.get(cur.fields.keys().next().expect(""));
                    }

                    cur = cur.get("next");
                }

                //todo: return a custom iterator
                instances.into_iter()
            }
            FieldInfo::ValueArray { len, .. } => {
                if field.is_primitive {
                    panic!(
                        "field '{}' is a primitive array, call the appropriate method. ({:?})",
                        name, field
                    );
                }

                if let Some(r#struct) = &self
                    .dna
                    .structs
                    .iter()
                    .find(|s| s.type_index == field.type_index)
                {
                    let r#type = &self.dna.types[r#struct.type_index as usize];
                    let fields = generate_fields(r#struct, r#type, self.dna, &self.blend.header);

                    let data = self.data.data();
                    let mut instances = Vec::new();
                    for i in 0..len as usize {
                        let data_len = (data.len() / len) as usize;
                        let data_start = i * data_len;

                        instances.push(Instance {
                            dna: &self.dna,
                            blend: &self.blend,
                            type_name: r#type.name.clone(),
                            data: InstanceDataFormat::Raw(&data[data_start..data_start + data_len]),
                            fields: fields.clone(),
                        });
                    }

                    instances.into_iter()
                } else {
                    unreachable!("no type information found")
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
                                type_name: r#type.name.clone(),
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
                        let (fields, r#type) = generate_subsidiary_fields(
                            &self.dna,
                            &self.blend.header,
                            &field,
                            *dna_index,
                        )
                        .expect("");

                        let mut instances = Vec::new();
                        for i in 0..data.count as usize {
                            let data_len = (data.data.len() / data.count) as usize;
                            let data_start = i * data_len;

                            instances.push(Instance {
                                dna: &self.dna,
                                blend: &self.blend,
                                type_name: r#type.name.clone(),
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
                                    //panic!("field '{}' has a null element. '{:?}'", name, field)
                                    continue;
                                }
                                Some(address) => address,
                            }
                        }
                        _ => unimplemented!(),
                    };

                    let block = self.blend.blocks.iter().find(|b| match b {
                        Block::Principal { memory_address, .. }
                        | Block::Subsidiary { memory_address, .. } => *memory_address == address,
                        _ => false, //unimplemented!("{:?} {:?}", b, field),
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
                                type_name: r#type.name.clone(),
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
            FieldInfo::PointerArray {
                len,
                indirection_count,
                ..
            } if indirection_count == 1 => {
                let data = self.data.get(field.data_start, field.data_len);
                let pointer_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = data.len() / pointer_size;

                assert_eq!(len, pointer_count);

                let mut pointers = Vec::new();
                for i in 0..pointer_count {
                    let address = {
                        match self.parse_ptr_address(&data[i * pointer_size..]) {
                            None => {
                                //panic!("field '{}' has a null element. '{:?}'", name, field);
                                continue;
                            }
                            Some(address) => address,
                        }
                    };

                    let block = self.blend.blocks.iter().find(|b| match b {
                        Block::Principal { memory_address, .. }
                        | Block::Subsidiary { memory_address, .. } => *memory_address == address,
                        _ => false, //unimplemented!("{:?} {:?}", b, field),
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
                                type_name: r#type.name.clone(),
                                data: InstanceDataFormat::Block(
                                    block.expect("we are sure block is some here"),
                                ),
                                fields,
                            });
                        }
                        Some(Block::Subsidiary { dna_index, .. }) => {
                            if let Some((fields, r#type)) = generate_subsidiary_fields(
                                &self.dna,
                                &self.blend.header,
                                &field,
                                *dna_index,
                            ) {
                                pointers.push(Instance {
                                    dna: &self.dna,
                                    blend: &self.blend,
                                    type_name: r#type.name.clone(),
                                    data: InstanceDataFormat::Block(
                                        block.expect("we are sure block is some here"),
                                    ),
                                    fields,
                                });
                            }
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

    pub fn get_iter<T: AsRef<str>>(&self, name: T) -> impl Iterator<Item = Instance<'a>> {
        let name = name.as_ref();
        let field = self.expect_field(name);

        enum InstanceIterator<'b> {
            ListBase {
                last_address: NonZeroU64,
                cur: Instance<'b>,
                ended: bool,
            },
            ValueArray {
                dna: &'b Dna,
                blend: &'b RawBlend,
                fields: LinkedHashMap<String, FieldTemplate>,
                data: &'b [u8],
                len: usize,
                type_name: String,
                cur_index: usize,
            },
            Pointer1 {
                dna: &'b Dna,
                blend: &'b RawBlend,
                fields: LinkedHashMap<String, FieldTemplate>,
                data: &'b BlockData,
                type_name: String,
                cur_index: usize,
            },
            Pointer2 {
                dna: &'b Dna,
                blend: &'b RawBlend,
                pointers: std::vec::IntoIter<NonZeroU64>,
                field: FieldTemplate,
            },
        }

        impl<'b> Iterator for InstanceIterator<'b> {
            type Item = Instance<'b>;
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    InstanceIterator::ListBase {
                        ref last_address,
                        ref mut cur,
                        ref mut ended,
                    } => {
                        if *ended {
                            return None;
                        }

                        let ret = cur.clone();

                        if ret.memory_address() == *last_address {
                            *ended = true;
                            Some(ret)
                        } else {
                            while !cur.is_valid("next") {
                                *cur = cur.get(cur.fields.keys().next().expect(""));
                            }
                            *cur = cur.get("next");
                            Some(ret)
                        }
                    }
                    InstanceIterator::ValueArray {
                        ref dna,
                        ref blend,
                        ref fields,
                        ref data,
                        ref len,
                        ref type_name,
                        ref mut cur_index,
                    } => {
                        let data_len = (data.len() / len) as usize;
                        let data_start = *cur_index * data_len;

                        if data_start == data.len() {
                            return None;
                        }

                        *cur_index += 1;

                        Some(Instance {
                            dna,
                            blend,
                            type_name: type_name.clone(),
                            data: InstanceDataFormat::Raw(&data[data_start..data_start + data_len]),
                            fields: fields.clone(),
                        })
                    }
                    InstanceIterator::Pointer1 {
                        ref dna,
                        ref blend,
                        ref fields,
                        ref data,
                        ref type_name,
                        ref mut cur_index,
                    } => {
                        let data_len = (data.data.len() / data.count) as usize;
                        let data_start = *cur_index * data_len;

                        if data_start == data.data.len() {
                            return None;
                        }

                        *cur_index += 1;

                        Some(Instance {
                            dna,
                            blend,
                            type_name: type_name.clone(),
                            data: InstanceDataFormat::Raw(
                                &data.data[data_start..data_start + data_len],
                            ),
                            fields: fields.clone(),
                        })
                    }
                    InstanceIterator::Pointer2 {
                        ref blend,
                        ref dna,
                        ref mut pointers,
                        ref field,
                    } => {
                        for address in pointers {
                            let block = blend.blocks.iter().find(|b| match b {
                                Block::Principal { memory_address, .. }
                                | Block::Subsidiary { memory_address, .. } => {
                                    *memory_address == address
                                }
                                _ => false,
                            });

                            match block {
                                Some(Block::Principal { dna_index, .. }) => {
                                    let r#struct = &dna.structs[*dna_index];
                                    let r#type = &dna.types[r#struct.type_index];

                                    let fields =
                                        generate_fields(r#struct, r#type, &dna, &blend.header);

                                    return Some(Instance {
                                        dna: &dna,
                                        blend: &blend,
                                        type_name: r#type.name.clone(),
                                        data: InstanceDataFormat::Block(
                                            block.expect("we are sure block is some here"),
                                        ),
                                        fields,
                                    });
                                }
                                Some(Block::Subsidiary { dna_index, .. }) => {
                                    if let Some((fields, r#type)) = generate_subsidiary_fields(
                                        dna,
                                        &blend.header,
                                        &field,
                                        *dna_index,
                                    ) {
                                        return Some(Instance {
                                            dna,
                                            blend,
                                            type_name: r#type.name.clone(),
                                            data: InstanceDataFormat::Block(
                                                block.expect("we are sure block is some here"),
                                            ),
                                            fields,
                                        });
                                    } else {
                                        continue;
                                    }
                                }
                                Some(_) => unimplemented!(),
                                None => continue,
                            }
                        }
                        None
                    }
                }
            }
        }

        match field.info {
            FieldInfo::Value => {
                if field.type_name != "ListBase" {
                    panic!("field '{}' cannot be read as a list. ({:?})", name, field)
                }

                let list_instance = self.get(name);

                let last_address = list_instance.get("last").memory_address();
                let cur = list_instance.get("first");

                InstanceIterator::ListBase {
                    last_address,
                    cur,
                    ended: false,
                }
            }
            FieldInfo::ValueArray { len, .. } => {
                if field.is_primitive {
                    panic!(
                        "field '{}' is a primitive array, call the appropriate method. ({:?})",
                        name, field
                    );
                }

                if let Some(r#struct) = &self
                    .dna
                    .structs
                    .iter()
                    .find(|s| s.type_index == field.type_index)
                {
                    let r#type = &self.dna.types[r#struct.type_index as usize];
                    let fields = generate_fields(r#struct, r#type, self.dna, &self.blend.header);

                    let data = self.data.data();

                    InstanceIterator::ValueArray {
                        dna: self.dna,
                        blend: self.blend,
                        fields,
                        data,
                        len,
                        type_name: r#type.name.clone(),
                        cur_index: 0,
                    }
                } else {
                    unreachable!("no type information found")
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

                match block {
                    Block::Principal {
                        data, dna_index, ..
                    } => {
                        let r#struct = &self.dna.structs[*dna_index];
                        let r#type = &self.dna.types[r#struct.type_index];

                        let fields =
                            generate_fields(r#struct, r#type, &self.dna, &self.blend.header);

                        InstanceIterator::Pointer1 {
                            dna: self.dna,
                            blend: self.blend,
                            fields,
                            data,
                            type_name: r#type.name.clone(),
                            cur_index: 0,
                        }
                    }
                    Block::Subsidiary {
                        data, dna_index, ..
                    } => {
                        let (fields, r#type) = generate_subsidiary_fields(
                            &self.dna,
                            &self.blend.header,
                            &field,
                            *dna_index,
                        )
                        .expect("");

                        InstanceIterator::Pointer1 {
                            dna: self.dna,
                            blend: self.blend,
                            fields,
                            data,
                            type_name: r#type.name.clone(),
                            cur_index: 0,
                        }
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
                    match block {
                        Block::Principal { data, .. } | Block::Subsidiary { data, .. } => {
                            match self.parse_ptr_address(&data.data[i * pointer_size..]) {
                                None => {
                                    continue;
                                }
                                Some(address) => pointers.push(address),
                            }
                        }
                        _ => unimplemented!(),
                    }
                }

                InstanceIterator::Pointer2 {
                    blend: &self.blend,
                    dna: &self.dna,
                    field: field.clone(),
                    pointers: pointers.into_iter(),
                }
            }
            FieldInfo::PointerArray {
                indirection_count,
                len,
                ..
            } if indirection_count == 1 => {
                let data = self.data.get(field.data_start, field.data_len);
                let pointer_size = self.blend.header.pointer_size.bytes_num();
                let pointer_count = data.len() / pointer_size;

                assert_eq!(len, pointer_count);

                let mut pointers = Vec::new();
                for i in 0..pointer_count {
                    match self.parse_ptr_address(&data[i * pointer_size..]) {
                        None => {
                            continue;
                        }
                        Some(address) => pointers.push(address),
                    }
                }

                InstanceIterator::Pointer2 {
                    blend: &self.blend,
                    dna: &self.dna,
                    field: field.clone(),
                    pointers: pointers.into_iter(),
                }
            }
            _ => unimplemented!(),
        }
    }
}

pub struct Blend {
    /// `blend` field contains the header, file-blocks and dna of the .blend file, which are used in runtime to
    /// interpret the blend file data.
    pub blend: RawBlend,
}

impl Blend {
    //todo: return io::Result
    pub fn from_path<T: AsRef<Path>>(path: T) -> Blend {
        use std::{fs::File, io::Cursor};

        let mut file = File::open(path).expect("could not open .blend file");

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("could not read .blend file");

        Blend::new(Cursor::new(buffer))
    }

    //todo: return result
    pub fn new<T: Read>(data: T) -> Blend {
        let blend = RawBlend::from_data(data).unwrap();
        Self { blend }
    }

    /// A blend file is made of blocks of binary data which represent structs. These blocks can have pointers to other
    /// blocks but only root blocks have a defined type (Object, Mesh, Materal, etc). Subsidiary blocks may or may not
    /// have the correct type information in their headers, but their type is defined by the field that accesses them.
    /// You can only query for root blocks because subsidiary blocks have to be accessed through some field for their
    /// type to be known.
    // todo: rename to root_blocks, return iterator
    // todo: rename to root_instances?
    pub fn get_all_root_blocks(&self) -> Vec<Instance> {
        self.blend
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Principal { dna_index, .. } /*| Block::Global { dna_index, .. }*/ => {
                    let dna_struct = &self.blend.dna.structs[*dna_index];
                    let dna_type = &self.blend.dna.types[dna_struct.type_index];

                    let fields =
                        generate_fields(dna_struct, dna_type, &self.blend.dna, &self.blend.header);

                    Some(Instance {
                        dna: &self.blend.dna,
                        blend: &self.blend,
                        type_name: dna_type.name.clone(),
                        data: InstanceDataFormat::Block(block),
                        fields,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    /// Root blocks have a code that tells us their type, "OB" for object, "ME" for mesh, "MA" for material, etc.
    /// You can use this method to filter for a single type of block.
    // todo: return iterator
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
                        type_name: r#type.name.clone(),
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

fn generate_subsidiary_fields<'a>(
    dna: &'a Dna,
    header: &BlendHeader,
    field: &FieldTemplate,
    dna_index: usize,
) -> Option<(LinkedHashMap<String, FieldTemplate>, &'a DnaType)> {
    if field.type_index >= 12 {
        if dna_index >= 12 {
            //assert_eq!(field.type_index as u16, self.dna.structs[block.header.sdna_index as usize].0);
            let r#struct = &dna.structs[dna_index];
            let r#type = &dna.types[r#struct.type_index];
            Some((generate_fields(r#struct, r#type, dna, header), r#type))
        } else if let Some(r#struct) = &dna
            .structs
            .iter()
            .find(|s| s.type_index == field.type_index)
        {
            let r#type = &dna.types[r#struct.type_index as usize];
            Some((generate_fields(r#struct, r#type, dna, header), r#type))
        } else {
            None
        }
    } else {
        let r#struct = &dna.structs[dna_index];
        if r#struct.type_index >= 12 {
            let r#type = &dna.types[r#struct.type_index];
            Some((generate_fields(r#struct, r#type, &dna, header), r#type))
        } else {
            None
        }
    }
}
