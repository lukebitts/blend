# blend_parse

This crates parses the file blocks and their headers in `blend` files from [Blender](https://www.blender.org/).

## The `.blend` file

A .blend is a binary file which starts with a header and has a number of "file-blocks". These file-blocks also contain a header and some binary data. A simple .blend file has over 2000 of these file-blocks.

    --------------------------------
    | header                       |
    --------------------------------
    | ---------------------------- |
    | | file-block header        | |
    | ---------------------------- |
    | | file-block data          | |
    | ---------------------------- |
    |                              |
    | ---------------------------- |
    | | file-block header        | |
    | ---------------------------- |
    | | file-block data          | |
    | ---------------------------- |
    |                              |
    |           [ ... ]            |
    --------------------------------

## More info

As you might have guessed, there is more to parsing a .blend file. This crates parses only the headers and file-blocks. If you have interest in also parsing the file-block data, try [blend_sdna](todo:add_link) and [blend](todo:add_link).

This crate does not support gzip-compressed .blend files, but if you decompress the file before trying to parse it, it should work.

 ## Example

 ```rust
use blend_parse::Blend;
use std::env;
use std::path;

fn main() {
    let base_path = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let blend_path = base_path.join("examples/userpref.blend");

    let blend = Blend::from_path(blend_path).unwrap();

    for block in blend.blocks {
        match &block.header.code {
            b"GLOB" => println!("GLOB"),
            b"DATA" => println!("DATA"),
            n => (),
        }
    }
}
 ```

