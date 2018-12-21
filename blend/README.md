# blend

This crate can be used to parse `blend` from [Blender](https://www.blender.org/) file and read the data of any internal structure: objects, images, materials and everything else. This crate is highly experimental and should not be used in production unless you understand the caveats around it. As a rule of thumb, __do not parse untrusted `blend` files__ as they can be manipulated in always that may cause this crate to panic, though no memory unsafety is expected.

## The `blend` file

The `blend` file has a complicated format. It is self-describing which means that some data inside the file is used to parse the rest of the data. This in turn means that a `blend` file is both backwards and forwards compatible, though in practice saving a `blend` file from one version of Blender and then opening in another version of Blender might not work. This crate on the other hand is (in theory) capable of reading files from any version of Blender.

Inside the `blend` file you can find a number of structures, the following example shows a Camera:


```rust
// Over 150 properties of the object were ommited
Object {
    id = {
        next = Pointer(Address(140576226520072))
        prev = Pointer(Null)
        name = "OBCamera"
        properties = Pointer(Null)
    }
    loc = [-61.23685, -79.32684, 63.045525]
    dloc = [0.0, 0.0, 0.0]
    orig = [0.0, 0.0, 0.0]
    size = [1.0, 1.0, 1.0]
    dsize = [0.0, 0.0, 0.0]
    dscale = [1.0, 1.0, 1.0]
    rot = [0.7853981, 0.00000000000000015700923, -0.887934]
    drot = [0.0, 0.0, 0.0]
    quat = [1.0, 0.0, 0.0, 0.0]
    dquat = [1.0, 0.0, 0.0, 0.0]
    rotAxis = [0.0, 1.0, 0.0]
    drotAxis = [0.0, 1.0, 0.0]
    rotAngle = 0.0
    drotAngle = 0.0
}
```

## This crate

While this crate tries to provide a sane API over the `blend` file, the `blend` is definetly not an interchange format. Blender basically dumps its entire memory into the disk and that's it. This means that structs inside the file can have primitive members, primitive array members, primitive multi-dimensional arrays, pointer to other structs, pointer to pointers, pointer arrays, function pointers, and a few other types. This crates provides a dynamic memory space which lets you read from these structs, deference pointers, read arrays at certains indices, etc. We try to provide a unified API over several internal concepts: `Blend::get_instances` can be used to read a list of structs whether they are behind a pointer or a pointer list for example.

## Caveats

### - This crate is really immature

Due to the complexity of emulating the C memory model in runtime there is a great chance you will find bugs. This is the 5th rewrite of this crate though and it has been used for non-trivial work successfully. Still, keep this in mind.

### - This crate is really opinionated

This crate was built to allow the use of `blend` files for game development and most design choices take this concept in account.

### - This crate panics at the first sign of misuse

To allow for an ergonomic API, many of the methods inside this library panic if certain conditions are not met when calling them. For instance, you can try to read an `i32` field as a `String`, this will panic instead of returning `None` or `Err(_)`.

### - Don't read untrusted files

As an extension of the previous point, it is possible to build a malicious `blend` file which causes this crate panic.

### - You need to know what exactly interests you in the `blend` file

Trying to access a property that doesn't exist panics. You need to know what exactly you need and the correct type. Some methods can be used to check if a property is valid before accessing it, but this doesn't cover everything. You can't check the type of a variable dinamically