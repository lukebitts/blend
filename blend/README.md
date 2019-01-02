# blend

This crate can be used to parse `.blend` from [Blender](https://www.blender.org/) file and read the data of any internal structure: objects, images, materials and everything else. __Do not parse untrusted `.blend` files__ as they can be written in ways that may cause this crate to panic.

## The `.blend` file

Inside the `.blend` file you can find a number of structures, the following example shows an `Object` and its corresponding `Camera` data:
    

```rust
{
    id: {
        name: "OBCamera"
        // ... 14 other properties omitted
    },
    loc: [7.3588915, -6.925791, 4.958309],
    size: [1.0, 1.0, 1.0],
    rot: [1.109319, 0.0, 0.8149282],
    quat: [1.0, 0.0, 0.0, 0.0],
    rotAxis: [0.0, 1.0, 0.0],
    rotAngle: 0.0,
    data: {
        id: {
            name: "CACamera",
            // ...
        },
        passepartalpha: 0.5,
        clipsta: 0.1,
        clipend: 100.0,
        lens: 50.0,
        drwfocusmat: [
            [0.0, 0.0, 0.0, 0.0], 
            [0.0, 0.0, 0.0, 0.0], 
            [0.0, 0.0, 0.0, 0.0], 
            [0.0, 0.0, 0.0, 0.0]
        ]
        // ...
    }
    // .. over 150 properties omitted
}
```

## Example

Considering the `Object` above, the following example shows how to read its properties:

```rust
let blend_file = Blend::from_path("path/to/file.blend");

// Blender uses 'OB' as the code for objects, see the
// documentation (todo:add_link) of Blend::get_by_code for other
// codes.
let mut objects = blend_file.get_by_code([b'O', b'B']);

let camera = objects
    .find(|object| object.get_instance("id").get_string("name") == "OBCamera")
    .unwrap();

let lens = camera.get_instance("data").get_f32("lens");

println!("{}", lens);
```

## This crate

The `.blend` file is a dump of Blender's memory at the time of saving. This means that a save file is simply a bunch of C-like structs and these structures can contain primitives (int, float, double, etc), arrays with one or more dimensions (int[4], float[4][4]), pointers (void\*, char\*, Material*), functions pointers, among others. 

What this crate tries to do is provide an API over this which simplifies a lot of concepts while still allowing for raw access if necessary. As an example, a `.blend` file has at least 2 concepts of a list: the first is in the way the structs are represented in the `.blend` file, they are inside 'file-blocks', but a 'file-block' can contain more than one struct. The vertices of a mesh are represented this way. The second list type is a linked-list, which contains `first*` and `last*` properties and these have a `prev*` and `next*` member. You can access both of these through the same `Blend::get_instances` function, and in the second case if you call `Blend::get_instance` you can access the `first*` and `last*` members yourself.

### Goals

The objective of this crate is to allow Blender to be used as an ad-hoc editor for a game I'm writing in Rust. This means `blend` expects the `.blend` file to always and exist always be valid. It also expects you to know in advance the types and names of the fields you need to access inside the structs, otherwise it panics. If you need to use this in production to read a `.blend` file for some reason, I recommend using this crate to preprocess your file into another simpler format and using that instead.

### This crate is really immature

Due to the complexity of emulating the C memory model in runtime there is a great chance you will find bugs. This is the 5th rewrite of this crate though and it has been used for non-trivial work successfully. Still, keep this in mind and please open an issue if something goes wrong.