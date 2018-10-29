extern crate blend_parse;
#[macro_use]
extern crate nom;

use blend_parse::{Endianness, PointerSize, Block};

#[derive(Debug)]
pub enum SdnaParseError {
    InvalidData,
    HeaderCodeIsNotDna1,
}

#[derive(Debug)]
pub struct Dna {
    pub names: Vec<String>,
    pub types: Vec<(String, u16)>,
    pub structs: Vec<(u16, Vec<(u16, u16)>)>,
}

impl Dna {
    pub fn from_sdna_block(dna_block: &Block, endianness: Endianness, pointer_size: PointerSize) -> Result<Dna, SdnaParseError> {
        if &dna_block.header.code != b"DNA1" {
            return Err(SdnaParseError::HeaderCodeIsNotDna1);
        }

        let parser = SdnaParseContext {
            endianness,
            pointer_size,
        };

        let res = parser.sdna(&dna_block.data);

        match res {
            (_, Ok((_, dna))) => {
                Ok(dna)
            }
            (_, Err(_)) => {
                Err(SdnaParseError::InvalidData)
            }
        }
    }
}

#[derive(Debug)]
pub struct SdnaParseContext {
    endianness: Endianness,
    pointer_size: PointerSize,
}

impl SdnaParseContext {
    method!(structs < SdnaParseContext, &[u8], Vec<(u16, Vec<(u16, u16)>)> >, self,
        do_parse!(
                tag!("STRC") >>
                structs_len: u32!(self.endianness.into()) >>
                structs: count!(
                    do_parse!(
                        struct_name_index: u16!(self.endianness.into()) >>
                        fields_num: u16!(self.endianness.into()) >>
                        fields: count!( do_parse!(
                            type_index: u16!(self.endianness.into()) >>
                            type_name: u16!(self.endianness.into()) >>
                            ( (type_index, type_name) )
                        ) ,fields_num as usize) >>
                        ( (struct_name_index, fields) )
                    ),
                    structs_len as usize) >>
                ( structs )
            )
    );

    method!(types < SdnaParseContext, &[u8], Vec<(String, u16)> >, self,
        do_parse!(
                tag!("TYPE") >>
                types_len: u32!(self.endianness.into()) >>
                types: count!(  
                    terminated!(
                        map!(map!(take_while!(|b: u8| b!=0), String::from_utf8_lossy), Into::into), tag!([0])
                    ), types_len as usize
                ) >>
                skip_len: value!({ 
                    let mut sum : usize = types.iter().map(|n : &String| -> usize { n.len() + 1 }).sum();
                    let res = sum;
                    while sum % 4 != 0 {
                        sum += 1;
                    }
                    sum - res
                }) >>
                take!(skip_len) >>
                tag!("TLEN") >>
                types_length: count!(  
                    u16!(self.endianness.into()), types_len as usize
                ) >>
                skip_len: value!({ 
                    let mut sum : usize = types_len as usize * 2;
                    let res = sum;
                    while sum % 4 != 0 {
                        sum += 1;
                    }
                    sum - res
                }) >>
                take!(skip_len) >>
                ( types.into_iter().zip(types_length).collect() )
            )
    );

    method!(names < SdnaParseContext, &[u8], Vec<String> >, self,
            do_parse!(
                tag!("NAME") >>
                names_len: u32!(self.endianness.into()) >>
                names: count!(  
                    terminated!(
                        map!(map!(take_while!(|b: u8| b!=0), String::from_utf8_lossy), Into::into), 
                        tag!([0])
                    ), names_len as usize
                ) >>
                skip_len: value!({ 
                    let mut sum : usize = names.iter().map(|n : &String| -> usize { n.len() + 1 }).sum();
                    let res = sum;
                    while sum % 4 != 0 {
                        sum += 1;
                    }
                    sum - res
                }) >>
                take!(skip_len) >>
                ( names )
            )
        );

    method!(pub sdna < SdnaParseContext, &[u8], Dna >, mut self,
            do_parse!(
                tag!("SDNA") >>
                names: call_m!(self.names) >>
                types: call_m!(self.types) >>
                structs: call_m!(self.structs) >>
                ( Dna { names, types, structs } )
            )
        );
}
