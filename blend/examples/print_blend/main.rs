extern crate blend;

use blend::Blend;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::{env, path};

pub fn main() -> Result<(), io::Error> {
    let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?);

    let blend_path = base_path.join("examples/print_blend/simple.blend");
    let output_path = base_path.join("examples/print_blend/output.txt");

    let mut file = File::open(blend_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let blend = Blend::new(&data[..]);
    let mut buffer = BufWriter::new(File::create(output_path)?);

    for (_struct_addr, ref struct_instance) in &blend.instance_structs {
        let instance_string: Vec<_> = struct_instance.to_string(0).bytes().collect();
        buffer.write(&instance_string[..])?;
        buffer.write(&b"\n"[..])?;
    }

    buffer.flush()?;

    Ok(())
}
