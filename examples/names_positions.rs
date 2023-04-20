use blend::Blend;
use std::{env, path};

fn print_names_and_positions(file_name: &str) {
    let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );
    let blend_path = base_path.join(format!("examples/blend_files/{}", file_name));
    let blend = Blend::from_path(blend_path).expect("error loading blend file");

    for obj in blend.instances_with_code(*b"OB") {
        let loc = obj.get_f32_vec("loc");
        let name = obj.get("id").get_string("name");

        println!("\"{}\" at {:?}", name, loc);
    }
}

fn main() {
    print_names_and_positions("2_80.blend");
    print_names_and_positions("2_90.blend");
    print_names_and_positions("3_0.blend");
    print_names_and_positions("3_5.blend");
}
