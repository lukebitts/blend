# blend_sdna

This crates parses the DNA of a `blend` file from [Blender](https://www.blender.org/).

## A `blend` file

A `blend` file is a self-describing format. It has a bunch of binary blocks and one of these blocks has the information necessary to parse the other blocks. This special block is called the DNA block.

## The DNA of a `blend` file

The DNA block contains the definition of many C struct types, it has type names, sizes in bytes, struct members, etc, and this can be used to parse the rest of the binary blocks.

## This crate

This crate parses the DNA block and expects a `Block` from the [blend_parse crate](todo:add_link) or raw binary data. The returned DNA type has the following format:

```rust
pub struct Dna {
    pub names: Vec<String>,
    pub types: Vec<(String, u16)>,
    pub structs: Vec<(u16, Vec<(u16, u16)>)>,
}
```
This struct represents the DNA data exactly as it is represented inside the `blend` file. Read the [documentation](todo:add_link) on the type to understand what each field means.

## Example

### Using the `blend_parse` crate:

 ```rust
fn main() {
    let blend = match Blend::from_path("path_to_your.blend") {
            Ok(blend) => blend,
            Err(BlendParseError::Io(_)) => panic!("File could not be opened"),
            Err(BlendParseError::InvalidData) => panic!("File could not be parsed correctly"),
        }
    };

    let dna = {
        // The last block is always the DNA block and has a block.code equal to b"DNA1"
        let dna_block = &blend.blocks[blend.blocks.len() - 1];
        match Dna::from_sdna_block(
            dna_block,
            blend.header.endianness,
            blend.header.pointer_size,
        ) {
            Ok(dna) => dna,
            Err(SdnaParseError::HeaderCodeIsNotDna1) => panic!("Block code is not DNA1"),
            Err(SdnaParseError::InvalidData) => panic!("Block could not be parsed as DNA"),
        }
    };

    for (struct_type_index, _struct_fields) in &dna.structs {
        let (_struct_type_name, _struct_type_bytes_len) = 
            &dna.types[*struct_type_index as usize];
    }
}
 ```