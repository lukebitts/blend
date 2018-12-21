extern crate blend;

use blend::Blend;
use std::fs::File;
use std::io::{self, Read, Write};
use std::{env, path};

pub fn application_root_dir() -> Result<path::PathBuf, io::Error> {
    if let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR") {
        return Ok(path::PathBuf::from(manifest_dir));
    }

    let mut exe = env::current_exe()?.canonicalize()?;

    // Modify in-place to avoid an extra copy.
    if exe.pop() {
        return Ok(exe);
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Failed to find an application root",
    ))
}

pub fn main() -> Result<(), io::Error> {
    let blend_path = application_root_dir()?.join("examples/print_blend/simple.blend");
    let output_path = application_root_dir()?.join("examples/print_blend/output.txt");

    let mut file = File::open(blend_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let blend = Blend::new(&data[..]);
    let mut buffer = File::create(output_path)?;

    for (_struct_addr, ref struct_instance) in &blend.instance_structs {
        let instance_string: Vec<_> = struct_instance.to_string(0).bytes().collect();
        buffer.write(&instance_string[..])?;
        buffer.write(&b"\n"[..])?;
    }

    Ok(())
}
