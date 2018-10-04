extern crate blend_sdna;

use blend_sdna::sdna::{parse_blend_file, Blend as RawBlend, Block, StructureTemplate};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Read};
use std::path::Path;

#[derive(Debug)]
pub struct Blend {
    raw_blend: RawBlend,
    memory: HashMap<u64, Block>,
}

impl Blend {
    pub fn from_path<T: AsRef<Path>>(path: T) -> io::Result<Blend> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut raw_blend = parse_blend_file(&mut Cursor::new(buffer))?;
        let mut blocks = std::mem::replace(&mut raw_blend.blocks, Vec::new());

        let memory = blocks
            .drain(..)
            .map(|b| (b.header.old_memory_address, b))
            .collect();

        Ok(Blend { raw_blend, memory })
    }

    fn block(&self, addr: u64) -> Option<&Block> {
        self.memory.get(&addr)
    }

    pub fn instance(&self, addr: u64) -> Option<()> {
        let block = self.block(addr)?;

        //We skip some specific codes which have invalid SDNA indexes.
        match block.header.code {
            [68, 65, 84, 65] => return None, //DATA block
            [82, 69, 78, 68] => return None, //REND block
            [84, 69, 83, 84] => return None, //TEST block
            _ => (),
        }

        let template = self.raw_blend.dna.structures.get(block.header.sdna_index)?;
        let block_type = self.raw_blend.dna.types.get(template.type_index)?;

        assert_eq!(
            block_type.length as usize,
            block.data.len() / block.header.count as usize,
            "> {:?}\n{:?}\n{:?}",
            block.header,
            template,
            block_type
        );

        None
    }
}

pub fn main() {
    let blend = Blend::from_path("..\\leaf\\assets\\simple.blend").unwrap();

    for (addr, _) in &blend.memory {
        blend.instance(*addr);
    }

    //println!("{:?}", blend.instance(2536418621560));
}
