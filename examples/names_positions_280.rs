use blend::Blend;
use std::{env, path};

fn main() {
    let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );
    let blend_path = base_path.join("examples/blend_files/2_80.blend");
    let blend = Blend::from_path(blend_path);

    for obj in blend.get_by_code(*b"OB") {
        let loc = obj.get_f32_vec("loc");
        let name = obj.get("id").get_string("name");

        println!("\"{}\" at {:?}", name, loc);
    }
}
