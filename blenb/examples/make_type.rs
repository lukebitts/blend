use blenb::parser::Blend;
use blenb::sdna::Dna;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::{env, path};

fn main() -> Result<(), io::Error> {
    /*let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let blend_path = base_path.join("examples/simple.blend");*/

    let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );

    let blend_path = base_path.join("../assets/scenes/color_sketch/color.blend"); //examples/print_blend/simple.blend
    let output_path = base_path.join("examples/blend_types.txt");

    let blend = Blend::from_path(blend_path).unwrap();

    let dna = {
        let dna_block = &blend.blocks[blend.blocks.len() - 1];
        Dna::from_sdna_block(
            dna_block,
            blend.header.endianness,
            blend.header.pointer_size,
        )
        .unwrap()
    };

    let mut types = String::new();

    let mut index = 0;
    for (struct_type_index, struct_fields) in &dna.structs {
        let (struct_type_name, struct_type_size) = &dna.types[*struct_type_index as usize];

        types.push_str(&format!("(u:{})(i:{}) {} ({} bytes) {{\n", struct_type_index, index, struct_type_name, struct_type_size));
        index += 1;

        for (struct_field_type_index, struct_field_name_index) in struct_fields {
            let struct_field_name = &dna.names[*struct_field_name_index as usize];
            let (struct_field_type_name, struct_field_type_size) =
                &dna.types[*struct_field_type_index as usize];

            types.push_str(&format!(
                "\t(t:{}) {} {} ({} bytes);\n",
                struct_field_type_index, struct_field_type_name, struct_field_name, struct_field_type_size
            ));

        }

        types.push_str("}\n\n");
    }

    let mut buffer = BufWriter::new(File::create(output_path)?);
    let types_bytes: Vec<_> = types.bytes().collect();
    buffer.write(&types_bytes[..])?;
    buffer.write(&b"\n"[..])?;

    buffer.flush()?;

    Ok(())
}
