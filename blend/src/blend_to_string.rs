use crate::{
    parsers::{blend::Block, field::FieldInfo},
    runtime::{FieldTemplate, Instance, InstanceDataFormat, PointerInfo},
    Blend,
};
use std::{
    collections::{HashSet, VecDeque},
    num::NonZeroU64,
};

impl Blend {
    /// Returns a string representation of the entire blend file. A small blend file returns a 2mb string.
    pub fn to_string(&self) -> String {
        // This function has to identify the correct type of every struct and their fields to transform everything
        // correctly into a string. It serves as an example of how to read the data provided by this crate field by
        // field, how to identify their type and whether it is a valid field or not. While the recommended way of
        // using this crate is to know 100% what you want to access, `Blend::to_string` shows how a more exploratory 
        // approach could work.

        // Before converting everything to a string, we need to do some bookkeeping:
        // We could be printing a single or a many instances, this is necessary because a blend file has many
        // ways of representing an array. By using `InstanceNumber` we make this is explicit.
        enum InstanceNumber<'a> {
            Single(Instance<'a>),
            Many(Vec<Instance<'a>>),
        }

        // We can't just go through each `Instance` and print them. Root instances can be printed easily as
        // all the information we need is contained within their internal `Block`s. Subsidiary blocks on the other
        // hand can only have their type known once they are accessed through a field. So we need to go through every
        // field in every Instance recursively to know how to print everything.
        enum InstanceToPrint<'a> {
            Root(Instance<'a>),
            FromField {
                address: Option<NonZeroU64>,
                ident: usize,
                print_id: usize,
                field_template: FieldTemplate,
                instance: InstanceNumber<'a>,
            },
        }

        let root_blocks = self.get_all_root_blocks();
        // To avoid duplication we keep the address of the blocks we have seen.
        let mut seen_addresses: HashSet<_> = root_blocks
            .iter()
            .map(|root_block| {
                root_block
                    .data
                    .memory_address()
                    .expect("root blocks always have an old address")
            })
            .collect();

        // All root blocks are converted to an `InstanceToPrint`.
        let mut instances_to_print: VecDeque<_> =
            root_blocks.into_iter().map(InstanceToPrint::Root).collect();

        let mut final_string = String::new();
        let mut field_instance_print_id = 0_usize;

        // Converts a single field into a String and makes sure all the bookkeeping is correct and identation too.
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
            match &field_template.info {
                // A value field is easy to convert. If they are a primitive we simply emit the correct string,
                // if the field is not a primitive, we add an `InstanceToPrint::FromField` to the `instance_to_print`
                // queue.
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
                            instances_to_print.push_back(InstanceToPrint::FromField {
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

                    format!(
                        "{}    {}: {} = {}\n",
                        ident_string,
                        field_name,
                        field_template.type_name,
                        value_str.trim_end()
                    )
                }
                // A value array goes through the same process as a value, except for char arrays which are shown as
                // strings.
                FieldInfo::ValueArray { dimensions_len, .. } => {
                    let value_str = match &field_template.type_name[..] {
                        // Here we assume that every char array is a string, but blender also uses these for bitfields.
                        // todo: add the other values
                        "char" => instance.get_string(field_name),
                        _ => {
                            return format!(
                                "{}    {}: {}{:?} = [xyzabc]\n",
                                ident_string, field_name, field_template.type_name, dimensions_len,
                            )
                        }
                    };

                    format!(
                        "{}    {}: {}{:?} = \"{}\"\n",
                        ident_string,
                        field_name,
                        field_template.type_name,
                        dimensions_len,
                        value_str.trim_end()
                    )
                }
                // Pointers are also easy to convert, we follow their address and add a `InstanceToPrint::FromField` to
                // the queue.
                FieldInfo::Pointer {
                    indirection_count: 1,
                } => {
                    // This is a big assumption we make: A type index with 12 as its value is what Blender calls a Link,
                    // this type as far as I could understand breaks rules that every other block follows including
                    // lying about its own size and the type of its fields. We simply give up here.
                    if field_template.type_index == 12 {
                        return format!(
                            "{}    {}: *{} = {}\n",
                            ident_string, field_name, field_template.type_name, "/LINK/",
                        );
                    }

                    let pointer = instance.get_ptr(field_template);

                    // Here it is all a matter of finding out if the pointer points somewhere and adding another
                    // `InstanceToPrint::FromField` to the stack, if the `Instance` has already been seen we emit
                    // the pointer address instead.
                    let value_str = match pointer {
                        PointerInfo::Invalid => String::from("invalid"),
                        PointerInfo::Null => String::from("null"),
                        PointerInfo::Block(Block::Principal {
                            memory_address,
                            data,
                            ..
                        })
                        | PointerInfo::Block(Block::Subsidiary {
                            memory_address,
                            data,
                            ..
                        }) => {
                            if seen_addresses.contains(memory_address) {
                                format!("@{}", memory_address)
                            } else {
                                if data.count == 1 {
                                    instances_to_print.push_back(InstanceToPrint::FromField {
                                        address: Some(*memory_address),
                                        ident: ident + 1,
                                        print_id: *field_instance_print_id,
                                        field_template: field_template.clone(),
                                        instance: InstanceNumber::Single(instance.get(field_name)),
                                    });
                                } else {
                                    instances_to_print.push_back(InstanceToPrint::FromField {
                                        address: Some(*memory_address),
                                        ident: ident + 1,
                                        print_id: *field_instance_print_id,
                                        field_template: field_template.clone(),
                                        instance: InstanceNumber::Many(
                                            instance.get_vec(field_name).collect(),
                                        ),
                                    });
                                }

                                seen_addresses.insert(*memory_address);

                                *field_instance_print_id += 1;

                                format!("{{{}}}", *field_instance_print_id - 1)
                            }
                        }
                        PointerInfo::Block(_) => unimplemented!(),
                    };

                    format!(
                        "{}    {}: *{} = {}\n",
                        ident_string, field_name, field_template.type_name, value_str,
                    )
                }
                //todo: 
                _ => format!(
                    "{}    {}: {} = [xxx]\n",
                    ident_string, field_name, field_template.type_name
                ),
            }
        }

        while let Some(to_print) = instances_to_print.pop_front() {
            match to_print {
                InstanceToPrint::Root(instance) => match instance.data {
                    InstanceDataFormat::Block(block) => match block {
                        Block::Principal {
                            dna_index,
                            code,
                            memory_address,
                            ..
                        } => {
                            let dna_struct = &self.blend.dna.structs[*dna_index];
                            let dna_type = &self.blend.dna.types[dna_struct.type_index];

                            let block_code = String::from_utf8_lossy(code);
                            final_string.push_str(&format!(
                                "{} (code: {:?}) (address: {})\n",
                                dna_type.name, block_code, memory_address
                            ));

                            for (field_name, field_template) in &instance.fields {
                                final_string.push_str(&field_to_string(
                                    field_name,
                                    field_template,
                                    &instance,
                                    0,
                                    &mut field_instance_print_id,
                                    &mut instances_to_print,
                                    &mut seen_addresses,
                                ));
                            }
                        }
                        _ => unimplemented!(),
                    },
                    InstanceDataFormat::Raw(_) => {
                        unreachable!("root blocks data is always InstanceDataFormat::Block")
                    }
                },
                InstanceToPrint::FromField {
                    address,
                    ident,
                    print_id,
                    field_template,
                    instance,
                } => {
                    let mut field_string = if let Some(address) = address {
                        format!("{} (address: {})\n", field_template.type_name, address)
                    } else {
                        format!("{}\n", field_template.type_name)
                    };

                    match instance {
                        InstanceNumber::Single(instance) => {
                            for (field_name, field_template) in &instance.fields {
                                field_string.push_str(&field_to_string(
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
                            let ident_string: String =
                                std::iter::repeat("    ").take(ident).collect();
                            if let Some(instance) = instances.first() {
                                field_string.push_str(&format!("{}{{\n", ident_string));
                                for (field_name, field_template) in &instance.fields {
                                    field_string.push_str(&field_to_string(
                                        field_name,
                                        field_template,
                                        &instance,
                                        ident,
                                        &mut field_instance_print_id,
                                        &mut instances_to_print,
                                        &mut seen_addresses,
                                    ));
                                }
                                field_string = field_string.trim_end().to_string();
                                field_string.push_str(&format!("{ident_string}\n{ident_string}{ident_string}> and other {len} elements ... \n{ident_string}}}\n", 
                                    ident_string=ident_string,
                                    len=instances.len() - 1,
                                ));
                            }
                        }
                    }

                    final_string = final_string.replacen(
                        &format!("{{{}}}", print_id),
                        &field_string.trim_end(),
                        1,
                    );
                }
            }
        }

        final_string
    }
}
