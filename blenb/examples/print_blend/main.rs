use blenb::Blend;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::{env, path};

pub fn main() -> Result<(), io::Error> {
    let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );

    let blend_path = base_path.join("../assets/scenes/color_sketch/color.blend"); //examples/print_blend/simple.blend
    let output_path = base_path.join("examples/print_blend/output.txt");

    println!("{:?}", blend_path);

    let mut file = File::open(blend_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let blend = Blend::new(&data[..]);
    let mut buffer = BufWriter::new(File::create(output_path)?);

    let instance_string: Vec<_> = blend.to_string().bytes().collect();
    buffer.write(&instance_string[..])?;
    buffer.write(&b"\n"[..])?;

    buffer.flush()?;

    Ok(())
}
