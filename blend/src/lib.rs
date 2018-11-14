#![feature(never_type, concat_idents, nll)]

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
mod struct_parser;

use blend_parse::Blend as ParsedBlend;
use blend_sdna::Dna;
use field_parser::{parse_field, FieldInfo};
use primitive_parsers::parse_f32;
//use std::collections::HashMap;
use linked_hash_map::LinkedHashMap as HashMap;
use std::rc::Rc;
use struct_parser::{
    block_to_struct, BlendPrimitive, FieldInstance, FieldTemplate, PointerInfo, StructData,
    StructInstance,
};

pub struct Blend {
    blend: ParsedBlend,
    //dna: Dna,
    //templates: HashMap<u16, Vec<FieldTemplate>>,
    instance_structs: HashMap<u64, Rc<StructInstance>>,
}

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct Instance<'a> {
    #[derivative(Debug = "ignore")]
    blend: &'a Blend,
    pub instance: Rc<StructInstance>,
}

impl<'a> Instance<'a> {
    pub fn code(&self) -> [u8; 2] {
        self.instance.code.unwrap()
    }

    pub fn get_i32<T: AsRef<str>>(&self, name: T) -> i32 {
        //println!("{:?}", self.instance.type_name);
        //println!("<!> {}", name.as_ref());
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                //println!("\t<!> {:?}", field);

                match field {
                    FieldInstance::Value(BlendPrimitive::Int(v)) => *v,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_f32<T: AsRef<str>>(&self, name: T) -> f32 {
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Value(BlendPrimitive::Float(v)) => *v,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_string<T: AsRef<str>>(&self, name: T) -> String {
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Value(BlendPrimitive::CharArray1D(v)) => v
                        .iter()
                        .take_while(|c| **c != 0)
                        .map(|c| *c as u8 as char)
                        .collect::<String>(),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_f32_array<T: AsRef<str>>(&self, name: T) -> Vec<f32> {
        //println!("get_f32_array {}", name.as_ref());
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Value(BlendPrimitive::FloatArray1D(v)) => v.clone(),
                    FieldInstance::Pointer(PointerInfo::Address(addr, _)) => {
                        let instance = &self.blend.instance_structs[addr];

                        match instance.data {
                            StructData::Raw(ref data) => {
                                let f32_size = ::std::mem::size_of::<f32>();
                                assert!(data.len() % f32_size == 0);

                                data.chunks(f32_size)
                                    .map(|s| parse_f32(s, self.blend.blend.header.endianness))
                                    .collect()
                            }
                            _ => panic!(),
                        }
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_i16_array<T: AsRef<str>>(&self, name: T) -> Vec<i16> {
        //println!("get_f32_array {}", name.as_ref());
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Value(BlendPrimitive::ShortArray1D(v)) => v.clone(),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_instances<T: AsRef<str>>(&self, name: T) -> Vec<Instance<'a>> {
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];
                let mut ret = Vec::new();

                match field {
                    FieldInstance::PointerList(pointers) => {
                        for ptr in pointers {
                            match ptr {
                                PointerInfo::Address(addr, _) => {
                                    let instance = &self.blend.instance_structs[addr];

                                    ret.push(Instance {
                                        blend: self.blend,
                                        instance: instance.clone(),
                                    });
                                }
                                _ => panic!(),
                            }
                        }

                        ret
                    }
                    FieldInstance::Pointer(PointerInfo::Address(addr, _)) => {
                        let instance = &self.blend.instance_structs[addr];

                        match &instance.data {
                            StructData::List(instances) => {
                                for data in instances {
                                    ret.push(Instance {
                                        blend: self.blend,
                                        instance: Rc::new(StructInstance {
                                            type_name: String::from("[unknown]"),
                                            code: None,
                                            old_memory_address: None,
                                            data: StructData::Single(data.clone()),
                                        }),
                                    });
                                }
                            }
                            StructData::Single(data) => {
                                ret.push(Instance {
                                    blend: self.blend,
                                    instance: Rc::new(StructInstance {
                                        type_name: String::from("[unknown]"),
                                        code: None,
                                        old_memory_address: None,
                                        data: StructData::Single(data.clone()),
                                    }),
                                });
                            }
                            StructData::Raw(_data) => panic!(),
                        }
                        ret
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn is_valid<T: AsRef<str>>(&self, name: T) -> bool {
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Pointer(PointerInfo::Null)
                    | FieldInstance::Pointer(PointerInfo::Invalid) => false,
                    FieldInstance::Pointer(_) => true,
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn get_instance<T: AsRef<str>>(&self, name: T) -> Instance<'a> {
        match &self.instance.data {
            StructData::Single(instance) => {
                let field = &instance.fields[name.as_ref()];

                match field {
                    FieldInstance::Struct(data) => Instance {
                        blend: self.blend,
                        instance: Rc::new(StructInstance {
                            type_name: String::from("[unknown]"),
                            code: None,
                            old_memory_address: None,
                            data: StructData::Single(data.clone()),
                        }),
                    },
                    FieldInstance::Pointer(info) => match info {
                        PointerInfo::Address(addr, _) => Instance {
                            blend: self.blend,
                            instance: self.blend.instance_structs[addr].clone(),
                        },
                        _ => panic!(),
                    },
                    _ => panic!(),
                }
            }
            StructData::List(_) => panic!(),
            StructData::Raw(_) => panic!(),
        }
    }
}

impl Blend {
    pub fn new(data: &[u8]) -> Blend {
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
            let (_struct_type_name, struct_type_bytes_len) =
                &dna.types[*struct_type_index as usize];
            let mut fields = Vec::new();

            let mut data_start = 0;
            for (field_type_index, field_name_index) in struct_fields {
                let (field_type_name, field_type_bytes_len) =
                    &dna.types[*field_type_index as usize];
                let field_full_name = &dna.names[*field_name_index as usize];

                let is_primitive = *field_type_index < 12;
                let (_, (field_name, field_info)) =
                    parse_field(field_full_name).expect("field name could not be parsed");

                let field_bytes_len = match field_info {
                    FieldInfo::Pointer { .. } | FieldInfo::FnPointer => {
                        blend.header.pointer_size.bytes_num()
                    }
                    FieldInfo::PointerArray1D { len } => {
                        blend.header.pointer_size.bytes_num() * len
                    }
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

        let mut instance_structs = HashMap::new();
        let mut seen_addresses = std::collections::HashSet::new();

        for block in &blend.blocks {
            if block.header.code[2..=3] == [0, 0] {
                assert!(block.header.count == 1);

                let (struct_type_index, _) = &dna.structs[block.header.sdna_index as usize];
                let struct_template = &templates[struct_type_index];

                seen_addresses.insert(block.header.old_memory_address);

                let instance = Rc::new(block_to_struct(
                    &mut instance_structs,
                    &mut seen_addresses,
                    &templates,
                    Some(block.header.old_memory_address),
                    Some([block.header.code[0], block.header.code[1]]),
                    struct_template,
                    *struct_type_index as usize,
                    &blend,
                    &dna,
                    &block,
                ));

                instance_structs.insert(block.header.old_memory_address, instance);
            }
        }

        Blend {
            blend,
            //dna,
            //templates,
            instance_structs,
        }
    }

    pub fn get_by_code(&self, code: [u8; 2]) -> impl Iterator<Item = Instance> {
        self.instance_structs
            .iter()
            .filter(|(_, s)| s.code == Some(code))
            .map(|(_, s)| Instance {
                blend: &self,
                instance: s.clone(),
            }).collect::<Vec<Instance>>()
            .into_iter()
    }
}

pub fn first_last_to_vec<'a>(instance: Instance<'a>) -> Vec<Instance<'a>> {
    if !instance.is_valid("first") {
        return Vec::new();
    }

    let first = instance.get_instance("first");
    let last = instance.get_instance("last");

    let mut cur = first;
    let mut ret = Vec::new();

    loop {
        ret.push(cur.clone());

        if cur.instance.old_memory_address == last.instance.old_memory_address {
            break;
        }
        cur = cur.get_instance("next");
    }

    ret
}

pub fn main() {
    let mut file = File::open("assets/simple4.blend").unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let blend = Blend::new(&data[..]);

    for object in blend.get_by_code([b'O', b'B']) {
        println!("{}", object.get_instance("id").get_string("name"));

        if object.get_instance("data").instance.code == Some([b'M', b'E']) {
            let data = object.get_instance("data");
            let _polys = data.get_instances("mpoly");
            //let materials = data.get_instances("mat");

            println!("\t{}", data.get_instance("id").get_string("name"));

            blend_instance_to_mesh(&data);
        }
    }

    use std::fs::File;
    use std::io::{Read, Write};

    let mut buffer = File::create("hello2.txt").unwrap();

    for (_, ref s) in &blend.instance_structs {
        let d1: Vec<_> = s.to_string(0).bytes().collect();
        buffer.write(&d1[..]).unwrap();
        buffer.write(&b"\n"[..]).unwrap();
    }
}

pub fn blend_instance_to_mesh<'a>(mesh: &Instance<'a>) {
    let faces = mesh.get_instances("mpoly");
    let loops = mesh.get_instances("mloop");
    let uvs = mesh.get_instances("mloopuv");
    let verts = mesh.get_instances("mvert");

    let mut index_count = 0;

    let mut face_indice_count = 0;
    let mut face_indice_counta = 0;

    for face in &faces {
        let len = face.get_i32("totloop");
        let mut indexi = 1;

        face_indice_counta += len * 2 / 3;

        while indexi < len {
            face_indice_count += 3;
            indexi += 2;
        }
    }

    let mut face_buffer = vec![0u32; face_indice_count];
    let mut uv_buffer = vec![0f32; face_indice_count * 2];
    let mut normal_buffer = vec![0f32; face_indice_count * 3];
    let mut verts_array_buff = vec![0f32; face_indice_count * 3];

    for face in &faces {
        let len = face.get_i32("totloop");
        let start = face.get_i32("loopstart");
        let mut indexi = 1;
        let mut offset = 0;

        while indexi < len {
            let mut index = 0;

            for l in 0..3 {
                if (indexi - 1) + l < len {
                    index = start + (indexi - 1) + l;
                } else {
                    index = start;
                }

                let v = loops[index as usize].get_i32("v");
                let vert = &verts[v as usize];
                face_buffer[index_count] = index_count as u32;

                let co = vert.get_f32_array("co");
                verts_array_buff[index_count * 3 + 0] = co[0];
                verts_array_buff[index_count * 3 + 1] = co[2];
                verts_array_buff[index_count * 3 + 2] = -co[1];

                let no = vert.get_i16_array("no");
                normal_buffer[index_count * 3 + 0] = no[0] as f32 / 32767.0;
                normal_buffer[index_count * 3 + 1] = no[2] as f32 / 32767.0;
                normal_buffer[index_count * 3 + 2] = -no[1] as f32 / 32767.0;

                let uv = uvs[index as usize].get_f32_array("uv");
                let uv_x = uv[0];
                let uv_y = uv[1];
                uv_buffer[index_count * 2 + 0] = uv_x;
                uv_buffer[index_count * 2 + 1] = uv_y;

                index_count += 1;
            }

            indexi += 2;
        }
    }
    println!(
        "{:?}\n{:?}\n{:?}",
        verts_array_buff.chunks(3).collect::<Vec<_>>(),
        normal_buffer.chunks(3).collect::<Vec<_>>(),
        uv_buffer.chunks(2).collect::<Vec<_>>()
    );

    let xy = (&verts_array_buff[..])
        .chunks(3)
        .enumerate()
        .map(|(i, pos)| {
            (
                [pos[0], pos[1], pos[2]],
                [
                    normal_buffer[i * 3 + 0],
                    normal_buffer[i * 3 + 1],
                    normal_buffer[i * 3 + 2],
                ],
                [uv_buffer[i * 2 + 0], uv_buffer[i * 2 + 1]],
                [0., 0., 0.],
            )
        }).collect::<Vec<_>>();

    for c in xy.chunks(3) {
        let v0 = c[0].0;
        let v1 = c[1].0;
        let v2 = c[2].0;

        let uv0 = c[0].2;
        let uv1 = c[1].2;
        let uv2 = c[2].2;

        let n0 = c[0].1;

        let v2v1 = pos2.sub(pos1);
        let v3v1 = pos3.sub(pos1);

        let c2c1b = uv2.getY() - uv1.getY();
        let c3c1b = uv3.getY() - uv1.getY();

        let t = v2v1.mult(c3c1b).sub(v3v1.mult(c2c1b));

        let b = n.cross(t);

        let smoothTangent = b.cross(n).normalize();
    }
}
