# blend_parse

This crates parses the file blocks and their headers in `blend` files from [Blender](https://www.blender.org/).

## A `blend` file

A `blend` file is a binary format which, simplifying a lot, has a header with some information about how to parse the file and a bunch of binary "file-blocks" which also have headers. Something like this:


    --------------------------------
    | .blend header                |
    --------------------------------
    | ---------------------------- |
    | | file block header        | |
    | ---------------------------- |
    | | binary block data        | |
    | ---------------------------- |
    |                              |
    | ---------------------------- |
    | | file block header        | |
    | ---------------------------- |
    | | binary block data        | |
    | ---------------------------- |
    |                              |
    |           [ ... ]            |
    --------------------------------

These binary blocks represent C structs. You can have an `Object` block, a `Scene` block or a block representing the user settings. A simple `blend` file has over two thousand of these blocks.

## The DNA of a `blend` file

To fully parse the binary file-blocks you need to use a special block which is the DNA of the `blend` file. The DNA block gives you the definition of the C structs and can be used to transform a file-block into a `Camera` for example.

## This crate

This crate does not fully parse the file-blocks or the DNA block, all it does is parse the header of the file and the header of the file-blocks, you decide what to do with the binary data you get.

## Reading more data from the `blend` file

While this crate can't help you read the data inside the file-blocks, it is a building block for more complex use cases:

 * [blend_sdna](todo:add_link) can be used along with this crate to parse the DNA of the blend file.
 * [blend](todo:add_link) uses both `blend_sdna` and this crate to fully parse the `blend` file and the binary file-blocks, allowing access to the actual `Object`s or `Camera`s, etc.

 ## Example

 ```rust
fn main() {
    let blend = match Blend::from_path("path_to_your.blend") {
        Ok(blend) => blend,
        Err(e) => match e {
            BlendParseError::Io(_) => panic!("File could not be opened"),
            BlendParseError::InvalidData => panic!("File could not be parsed correctly"),
        }
    };

    for block in blend.blocks {
        match block.data {
            b"MA\0\0" => println!("A material block"),
            b"OB\0\0" => println!("An object"),
            b"CA\0\0" => println!("A camera"),
            b"DATA" => println!("A data block, type information comes from somewhere else"),
            _ => (),
        }
    }
}
 ```

