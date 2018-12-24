use blend_parse::Blend;
use std::env;
use std::path;

fn main() {
    let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let blend_path = base_path.join("examples/userpref.blend");

    let blend = Blend::from_path(blend_path).unwrap();

    for block in blend.blocks {
        match &block.header.code {
            b"GLOB" => println!("GLOB {}", block.header.old_memory_address),
            b"DATA" => println!("DATA {}", block.header.old_memory_address),
            n => (),
        }
    }
}
