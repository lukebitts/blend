//! # Blend - A crate for parsing .blend files from Blender
//! 
//! ## Example
//! 
//! ```ignore
//! use blend::Blend;
//! 
//! /// Prints the name and position of every object
//! fn main() {
//!     let blend = Blend::from_path("file.blend");
//! 
//!     for obj in blend.get_by_code(*b"OB") {
//!         let loc = obj.get_f32_vec("loc");
//!         let name = obj.get("id").get_string("name");
//! 
//!         println!("\"{}\" at {:?}", name, loc);
//!     }
//! }
//! ```
//! 
//! ## The .blend file
//! 
//! To use this crate to its full extent it is useful to understand how the .blend file works internally. A simplified
//! overview: Blender creates the save file by dumping its memory to the disk, this means that a .blend file is
//! a list of C-like structs which can contain primitives, arrays, pointers and other structs. The following is how a
//! Camera is defined in Blender's memory (in Rust-like syntax):
//! 
//! ```ignore
//! struct Camera {
//!     id: ID {
//!         name: [u8; 66] = "CACamera"
//!         //[... other ommited properties ...]
//!     },
//!     adt: *AnimData = null,
//!     type: u8 = 0,
//!     dtx: u8 = 0,
//!     flag: f32 = 4,
//!     passepartalpha: f32 = 0.5,
//!     clipsta: f32 = 0.1,
//!     clipend: f32 = 100,
//!     lens: f32 = 50,
//!     ortho_scale: f32 = 7.3142858,
//!     drawsize: f32 = 1,
//!     sensor_x: f32 = 36,
//!     sensor_y: f32 = 24,
//!     shiftx: f32 = 0,
//!     shifty: f32 = 0,
//!     YF_dofdist: f32 = 0,
//!     ipo: *Ipo = null,
//!     dof_ob: *Object = null,
//!     //[... other ommited properties ...]
//! }
//! ```
//! 
//! Other concepts are explained in the docs for methods where knowing these concepts is necessary.
//! 
//! ### Learn more
//! 
//! Documentation on the .blend file is a bit sparse, but the most common source is the [Mystery of the Blend](https://github.com/fschutt/mystery-of-the-blend-backup)
//! and a personal recommendation is to get it from the official Blender repository and apply the following [patch](https://developer.blender.org/T52387).
//! 
//! ## This crate
//! 
//! This crate provides a parser and a runtime for these structures which means you can access them as if they were
//! simple objects in memory. The aim here is to abstract most of the noise into a single coherent interface. For
//! example: you can access a struct through both a non-primitive value field and a non-null pointer field using the
//! same method call (`Instance::get`). Other abstractions are provided where possible: the .blend file has at least
//! 3 ways of defining lists of things, this crate provides a single method that unifies all of those.
//! 
//! This crate is also lazy. While some work has to be done upfront to find the structs and their type definitions,
//! the binary data of the blocks is not parsed until you actually access it.
//! 
//! ### Usage tips
//! 
//! Knowing what to read from the file can be a bit of a challenge. A simple .blend file has over 400 "blocks" and each can
//! represent one or more structs. If you don't know what you want to access exactly the `print_blend` example can be
//! helpful. You can use it to save an entire .blend file as text to disk. You can also print single struct instances
//! if you know somewhat what you need.
//! 
//! It's also important to note that when printing an `Instance` if one of their properties is a list, elements other
//! than the first are skipped. If you need to see the entire list simply access it and print its members individually.
//! 
//! The `Display` implementation for `Instance` is a bit unpolished so larger .blend
//! files might cause a stack overflow but that can be fixed by running the code in release mode. If you find something that
//! breaks formatting please open an issue.
//! 
//! ### Running examples
//! 
//! A .blend file may contain personal information from the machine it was created, that's why no .blend files are provided
//! in this repository. To run the examples create a folder named `blend_files` inside the `examples` folder, put any file
//! you want there and change the paths in the examples.
//! 
//! ### Supported versions
//! 
//! As the .blend file is self-describing it should possible to parse files from every Blender version (tests were done
//! on files from version 2.72 to 2.80). Some things are assumed to always be true though: the type `int` for example is
//! always considered equivalent to Rust's `i32` but there is nothing in the file specification that guarantees this. There
//! is very little reason to believe Blender would change its primitive types though.
//! 
//! ### Warnings
//! 
//! This crate is meant to be used with trusted .blend files, while no unsafety or undefined behaviour is expected
//! from reading a malicious file, it is probably possible to craft a .blend file that causes a panic at runtime.
//! 
//! Due to some quirks of the specification, parts of the
//! library are a bit liberal in what kind of conversions it allows. For example: any block with the correct
//! size can be parsed as a list of floats. Why? Because some blocks are actual arrays of floats but we don't have
//! enough type information to be sure of this. This means it is up to the user to decide what they want when accessing
//! the data.
//! 
//! This crate is also somewhat panic happy. While it should be always possible to check if the field you are accessing
//! exists, is valid, contains a particular type of data, etc, you are meant to know what you are accessing ahead of
//! time so almost none of the functions will return a `Result::Err` or `Option::None` on bad inputs.
//! 
//! Finally, this was developed to facilitate game development but should be useful for any use case.
//! 
//! ### Limitations
//! 
//! This crate does not support compressed .blend files and it also does not support writing .blend files. To solve the
//! first you can uncompress the file before passing the data to `blend::Blend::from_data` see the `print_blend` example
//! to see how. The second one is a bit harder due to the way the code is organized, but PRs are welcome!
//! 
//! `GLOB`, `REND` and `TEST` blocks are not fully supported. Parts of the code already supports these blocks but they are
//! not fully implemented as I haven't found a use-case for them. Open an issue if you would like support for these!


pub mod parsers;
pub mod runtime;

pub use runtime::{Blend, Instance};
