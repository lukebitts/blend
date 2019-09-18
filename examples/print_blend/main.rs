use blend::Blend;
use libflate::gzip::Decoder;
use std::{
    env,
    fs::File,
    io::{self, BufWriter, Read, Write},
    path::{self, PathBuf},
};
use walkdir::WalkDir;

fn do_it(file_name: impl AsRef<str>) -> Result<(), io::Error> {
    let file_name = file_name.as_ref();
    let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );

    let blend_path = base_path.join(format!("examples/blend_files/{}", file_name));
    let output_path = base_path.join(format!("examples/print_blend/output_{}.txt", file_name));

    println!("{}", blend_path.display());
    let mut file = File::open(blend_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if data[0..7] != *b"BLENDER" {
        let mut decoder = Decoder::new(&data[..])?;
        let mut gzip_data = Vec::new();
        decoder.read_to_end(&mut gzip_data)?;

        data = gzip_data;
    }

    let blend = Blend::new(&data[..]);
    let mut output_path_without_file = PathBuf::from(&output_path);
    output_path_without_file.pop();
    std::fs::create_dir_all(&output_path_without_file)?;
    let mut buffer = BufWriter::new(File::create(output_path)?);

    for o in blend.get_all_root_blocks() {
        let instance_string: Vec<_> = format!("{}", o)[..].bytes().collect();
        buffer.write_all(&instance_string[..])?;
    }

    buffer.write_all(&b"\n"[..])?;
    buffer.flush()?;

    println!("done: {}", file_name);

    Ok(())
}

pub fn main() -> Result<(), io::Error> {
    do_it("2_74.blend")?;
    /*do_it("2_77.blend")?;
    do_it("2_78.blend")?;
    do_it("2_79.blend")?;
    do_it("2_80.blend")?;*/

    //do_it("snake_cubes.blend")?;

    /*let base_path = path::PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("could not find cargo manifest dir"),
    );
    let production_path = base_path.join("examples/blend_files/production");
    let prefix_to_strip = base_path.join("examples/blend_files/");

    for entry in WalkDir::new(production_path) {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            let file_name = format!(
                "{}",
                entry
                    .path()
                    .strip_prefix(&prefix_to_strip)
                    .unwrap()
                    .display()
            );

            do_it(file_name)?;
        }
    }*/

    Ok(())
}
