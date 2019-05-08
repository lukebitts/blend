use blend_parse::Blend;
use blend_sdna::Dna;
use std::env;
use std::path;

fn main() {
    let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let blend_path = base_path.join("examples/simple.blend");

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

    for (struct_type_index, struct_fields) in &dna.structs {
        let (struct_type_name, struct_type_size) = &dna.types[*struct_type_index as usize];

        println!("{} ({} bytes) {{", struct_type_name, struct_type_size);

        for (struct_field_type_index, struct_field_name_index) in struct_fields {
            let struct_field_name = &dna.names[*struct_field_name_index as usize];
            let (struct_field_type_name, struct_field_type_size) =
                &dna.types[*struct_field_type_index as usize];

            println!(
                "\t{} {} ({} bytes);",
                struct_field_type_name, struct_field_name, struct_field_type_size
            );
        }

        println!("}}");
    }
}
