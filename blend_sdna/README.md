# blend_sdna

This crates parses the DNA of a `blend` file from [Blender](https://www.blender.org/).

## The `.blend` file

A .blend file is basically a dump of Blender's memory at the time of the save. Inside the file you will find several structs which can represent concepts like objects, cameras, ui preferences, images, and many others.

These structs initially exist only as binary data inside the file, in this state they are just file-blocks. One of these is the DNA block which can be used to parse the other blocks.

## The DNA

The DNA block describes how to parse the other file-blocks. The DNA contains the definition of the structs, their types, size in bytes, inner properties, etc. You can use the DNA as a template to parse the file-blocks.

The DNA type is the following:

```rust
pub struct Dna {
    pub names: Vec<String>,
    pub types: Vec<(String, u16)>,
    pub structs: Vec<(u16, Vec<(u16, u16)>)>,
}
```

The `types` list contains a tuple with the name of the type as the first member and the size in bytes as the seconds member.

Example:

```rust
let types = [
    ("char", 1),
    ("short", 2),
    ("int", 4),
    ("ListBase", 16),
    ("Mesh", 1560),
    ("DynamicPaintModifierData", 128),
    ("DynamicPaintCanvasSettings", 104),
    ...
];
``` 

The `structs` list contains a tuple where the first element is the index of the struct type in the `types` list and the second element is the struct's fields. Each field is a tuple where the first element is the index to the type in the `types` list and the second element is an index to the field name in the `names` list.

Considering the `types` variable defined above: 

```rust
let names = [
    "*pmd",
    "*mesh",
    "surfaces",
    "active_sur",
    "flags",
    "pad",
    "error[64]",
    ...
];

let structs = [
    (6, [              
        (5, 0),
        (4, 1),
        (3, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (0, 6),
    ])                 
]
```

The information above represents the following struct:

```
DynamicPaintCanvasSettings (104 bytes) {
    DynamicPaintModifierData *pmd (128 bytes);
    Mesh *mesh (1560 bytes);
    ListBase surfaces (16 bytes);
    short active_sur (2 bytes);
    short flags (2 bytes);
    int pad (4 bytes);
    char error[64] (1 bytes);
}
```

At first glance the sum of the fields' byte size is greater than the struct's byte size, but consider that `*pmd` and `*mesh` are both pointers, and actually have 8 bytes each. The same goes for the `error[64]` field which has 64 bytes instead of 1. 

## More info

This crate parses only the DNA block of the .blend file, and does so using the [blend_parse](todo:add_link) crate. 

If you want to use the DNA data to parse the rest of the .blend file you have at least two options. You could generate `#[repr(C)]` rust structs from this crate's output and then parse the file-blocks' binary data directly into these structs. The downside of this option is that you will need to generate the structs' definition for every Blender version you want to support.

Another option is to parse the file-blocks in runtime. The downside is a higher memory and processing usage. This can be done using the [blend](todo:add_link) crate.

## Example

```rust
use blend_parse::Blend;
use blend_sdna::Dna;
use std::env;
use std::path;

let blend = Blend::from_path("your_blend_file.blend").unwrap();
let dna = {
    let dna_block = &blend.blocks[blend.blocks.len() - 1];
    Dna::from_sdna_block(
        dna_block,
        blend.header.endianness,
        blend.header.pointer_size,
    )
    .unwrap()
};

for (struct_type_index, struct_fields) in &dna.structs {
    let (struct_type_name, struct_type_size) = &dna.types[*struct_type_index as usize];

    println!("{} ({} bytes) {{", struct_type_name, struct_type_size);

    for (struct_field_type_index, struct_field_name_index) in struct_fields {
        let struct_field_name = &dna.names[*struct_field_name_index as usize];
        let (struct_field_type_name, struct_field_type_size) =
            &dna.types[*struct_field_type_index as usize];

        println!(
            "\t{} {} ({} bytes);",
            struct_field_type_name, struct_field_name, struct_field_type_size
        );
    }

    println!("}}");
}
```
