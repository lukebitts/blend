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
    pub fn to_string(&self) -> String {
        enum InstanceNumber<'a> {
            Single(Instance<'a>),
            Many(Vec<Instance<'a>>),
        }

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
        let mut seen_addresses: HashSet<_> = root_blocks
            .iter()
            .map(|root_block| {
                root_block
                    .data
                    .memory_address()
                    .expect("root blocks always have an old address")
            })
            .collect();

        let mut instances_to_print: VecDeque<_> =
            root_blocks.into_iter().map(InstanceToPrint::Root).collect();

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
            match &field_template.info {
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
                FieldInfo::ValueArray { dimensions_len, .. } => {
                    let value_str = match &field_template.type_name[..] {
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
                FieldInfo::Pointer {
                    indirection_count: 1,
                } => {
                    if field_template.type_index == 12 {
                        return format!(
                            "{}    {}: *{} = {}\n",
                            ident_string, field_name, field_template.type_name, "/LINK/",
                        );
                    }

                    let pointer = instance.get_ptr(field_template);

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
