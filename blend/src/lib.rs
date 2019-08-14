//! # Blend - A crate for parsing .blend files from Blender
//! 
//! ## The .blend file
//! To use this crate to its full extent it is useful to understand the .blend file itself. The following is a 
//! simplified explanation of the format:
//! 
//! .blend files are binary files created by Blender through a process where Blender dumps its memory into a file which
//! can later be used to recreate the values in memory when loading the file. This ensures saving and loading is a
//! really fast process, though it certainly complicates things for us.
//! 
//! The .blend file, being a representation o Blender's memory, is basically a bunch of structs. A camera for example
//! has the following structure (using C-like syntax):
//! 
//! ```
//! struct Camera {
//!     ID id = {
//!         char[66] name = "CACamera"
//!         //[... other ommited properties ...]
//!     },
//!     *AnimData adt = null,
//!     char type = 0,
//!     char dtx = 0,
//!     short flag = 4,
//!     float passepartalpha = 0.5,
//!     float clipsta = 0.1,
//!     float clipend = 100,
//!     float lens = 50,
//!     float ortho_scale = 7.3142858,
//!     float drawsize = 1,
//!     float sensor_x = 36,
//!     float sensor_y = 24,
//!     float shiftx = 0,
//!     float shifty = 0,
//!     float YF_dofdist = 0,
//!     *Ipo ipo = null,
//!     *Object dof_ob = null,
//!     //[... other ommited properties ...]
//! }
//! ```
//! 
//! Notice how in that definition we have primitive fields, non-primitive fields and pointers.
//! 
//! ### The specification of the file
//! The .blend file contains A LOT of these structs, everything is represented by one of these, from cameras
//! to meshes to materials, from user settings to panel sizes and window configurations. How are these different types
//! represented in the binary file? Simplifying a lot, the file is made of "Blocks" which are simply a blob of binary
//! data and can mean anything. One of these blocks is the DNA block (sometimes called the SDNA of the file) and it
//! contains the type definitions needed to parse the other blocks. You can think of it this way, the DNA defines
//! structs and the other blocks are instances of these structs.
//! 
//! ### Primary (or root) and Subsidiary blocks
//! At the beggining of the file there is a block which is the "GLOB" ("globals") block, this block has pointer to other
//! blocks which in turn point to other blocks and so on, forming an acyclic directed graph (with a few exceptions).
//! 
//! A root block is defined as being the root of a major structure, an object, a camera, a material, etc. These blocks
//! can point to subsidiary blocks which in turn can point to other subsidiary blocks and so on. Any of these can also
//! point to other root blocks but subsidiary blocks always belong to a single root block and so no block has a pointer
//! to a subsidiary block that is not owned by its own root block.
//! 
//! In practical terms there are a few differences between both types of blocks. A major one is that in the .blend file
//! root blocks always have their correct type, that is, if you have a single root block you can parse the binary data
//! without any issue. Subsidiary blocks on the other hand can only be parsed if you know the type of the field 
//! accessing it. For example, if a block has a field of type `*CurveMapping` (pointer to a curve mapping), the pointed
//! block (if subsidiary) may or may not have the correct type, but you can be sure it is a `CurveMapping` because of
//! the field. Another difference is that root blocks always have a 2 digit code: "OB" for objects, "ME" for meshes,
//! "MA" for materials, etc. Subsidiary blocks always have the same 4 digit code "DATA".
//! 
//! ### Learn more
//! You can read the Mystery of the Blend to learn more about the file.
//! 
//! ## This crate
//! This crate provides a parser and a runtime for these structures which means you can access them as if they were
//! simple objects in memory. The aim here is to abstract most of the noise into a single coherent interface. For 
//! example: you can access a struct through both a non-primitive value field and a non-null pointer field using the
//! same method call. Other abstractions are provided where possible, the .blend file has at least 3 ways of defining
//! lists of things, this crate provides a single method that unifies all of those.
//! 
//! This crate is also lazy. While some work has to be done upfront to find the blocks and their type definitions,
//! the binary data of the blocks is not parsed until you actually access it.
//! 
//! ### Usage and warnings
//! This crate is meant to be used with trusted .blend files, while no unsafety or undefined behaviour is expected
//! from reading a malicious file, it is probably possible to craft a .blend file that causes a panic at runtime. 
//! 
//! Due to some quirks of the specification, parts of the
//! library are a bit liberal in what kind of conversions it allows. For example: any block with the correct
//! size can be parsed a list of floats, why? Because some subsidiary blocks are actual arrays of floats, but neither
//! the field accessing them has "array of floats" as type nor the block has the correct type (the field
//! is a pointer to void and the block has an invalid type). This means it is up to
//! the user to decide what they want when accessing the data.
//! 
//! 
//! This crate is also somewhat panic happy. While it should be always possible to check if the field you are accessing
//! exists, is valid, contains a particular type of data, etc, you are meant to know what you are accessing ahead of 
//! time so almost none of the functions will return a `Result::Err` or `Option::None` on bad inputs.
//! 
//! Finally, this was developed to facilitate game development but should be useful for any use case.
//! 
//! ### Limitations
//! This crate does not support compressed .blend files (though you can uncompress them before trying to read 
//! it) and it also does not support writing .blend files (as this would be a bit hard to do because of the current
//! way the code is organized). PRs for both features would be welcomed though!
//! 
//! ## Example
//! 

mod blend_to_string;
pub mod parsers;
pub mod runtime;

pub use runtime::{Blend, Instance};
