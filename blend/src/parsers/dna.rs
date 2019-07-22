use crate::parsers::{Endianness, PointerSize, Result};
use nom::{
    bytes::complete::{tag, take, take_while},
    combinator::map,
    multi::count,
    number::complete::{be_u16, be_u32, le_u16, le_u32},
    sequence::terminated,
};
use std::convert::TryInto;

#[derive(Debug)]
pub struct Dna {
    pub names: Vec<String>,
    pub types: Vec<DnaType>,
    pub structs: Vec<DnaStruct>,
}

#[derive(Debug)]
pub struct DnaType {
    pub name: String,
    pub bytes_len: usize, //size in bytes of the type
}

#[derive(Debug)]
pub struct DnaField {
    pub type_index: usize, //index on Dna::types array
    pub name_index: usize, //index on Dna::names array
}

#[derive(Debug)]
pub struct DnaStruct {
    pub type_index: usize, //index on Dna::types array
    pub fields: Vec<DnaField>,
}

#[derive(Debug)]
pub struct DnaParseContext {
    endianness: Endianness,
    pointer_size: PointerSize,
}

impl DnaParseContext {
    pub fn new(endianness: Endianness, pointer_size: PointerSize) -> Self {
        Self {
            endianness,
            pointer_size,
        }
    }

    /// Panics if a u32 can't be converted to usize in your system.
    fn names<'a, 'b>(&'a self, input: &'b [u8]) -> Result<'b, Vec<String>>
    where
        'b: 'a,
    {
        let (input, _) = tag("NAME")(input)?;
        let (input, names_len) = match self.endianness {
            Endianness::Little => le_u32(input)?,
            Endianness::Big => be_u32(input)?,
        };
        let all_names_len = std::cell::RefCell::new(0_usize);
        let (input, names) = count(
            terminated(
                map(take_while(|b: u8| b != 0), |b: &[u8]| {
                    *all_names_len.borrow_mut() += b.len() + 1; //+1 for the null terminating separator
                    String::from_utf8_lossy(b).into_owned()
                }),
                tag("\0"),
            ),
            names_len.try_into().expect("u32 to usize"),
        )(input)?;

        let skip_len = {
            let mut sum = *all_names_len.borrow();
            let res = sum;
            while sum % 4 != 0 {
                sum += 1;
            }
            sum - res
        };

        let (input, _) = take(skip_len)(input)?;

        Ok((input, names))
    }

    /// Panics if a u32 can't be converted to usize in your system.
    fn types<'a, 'b>(&'a self, input: &'b [u8]) -> Result<'b, Vec<DnaType>>
    where
        'b: 'a,
    {
        let (input, _) = tag("TYPE")(input)?;
        let (input, types_len) = match self.endianness {
            Endianness::Little => le_u32(input)?,
            Endianness::Big => be_u32(input)?,
        };

        let types_len = types_len.try_into().expect("u32 to usize");
        let all_type_names_len = std::cell::RefCell::new(0_usize);

        let (input, type_names) = count(
            terminated(
                map(take_while(|b: u8| b != 0), |b: &[u8]| {
                    *all_type_names_len.borrow_mut() += b.len() + 1;
                    String::from_utf8_lossy(b).into_owned()
                }),
                tag("\0"),
            ),
            types_len,
        )(input)?;

        let skip_len = {
            let mut sum = *all_type_names_len.borrow();
            let res = sum;
            while sum % 4 != 0 {
                sum += 1;
            }
            sum - res
        };
        let (input, _) = take(skip_len)(input)?;

        let (input, _) = tag("TLEN")(input)?;
        let (input, type_lenghts) = count(
            match self.endianness {
                Endianness::Little => le_u16,
                Endianness::Big => be_u16,
            },
            types_len,
        )(input)?;

        let skip_len = {
            let mut sum = types_len * 2;
            let res = sum;
            while sum % 4 != 0 {
                sum += 1;
            }
            sum - res
        };
        let (input, _) = take(skip_len)(input)?;

        Ok((
            input,
            type_names
                .into_iter()
                .zip(type_lenghts)
                .map(|(name, length)| DnaType {
                    name,
                    bytes_len: length.try_into().expect("u32 to usize"),
                })
                .collect(),
        ))
    }

    /// Panics if a u32 can't be converted to usize in your system.
    fn structs<'a, 'b>(&'a self, input: &'b [u8]) -> Result<'b, Vec<DnaStruct>> {
        let (input, _) = tag("STRC")(input)?;
        let (input, structs_len) = match self.endianness {
            Endianness::Little => le_u32(input)?,
            Endianness::Big => be_u32(input)?,
        };

        let mut structs = Vec::new();
        let mut final_input = input;
        for _ in 0..structs_len {
            let (input, struct_name_index) = match self.endianness {
                Endianness::Little => le_u16(final_input)?,
                Endianness::Big => be_u16(final_input)?,
            };
            let (input, fields_num) = match self.endianness {
                Endianness::Little => le_u16(input)?,
                Endianness::Big => be_u16(input)?,
            };

            let mut next_input = input;
            let mut fields = Vec::new();
            for _ in 0..fields_num {
                let (input, field_type_index) = match self.endianness {
                    Endianness::Little => le_u16(next_input)?,
                    Endianness::Big => be_u16(next_input)?,
                };
                let (input, field_name_index) = match self.endianness {
                    Endianness::Little => le_u16(input)?,
                    Endianness::Big => be_u16(input)?,
                };
                next_input = input;

                fields.push(DnaField {
                    type_index: field_type_index.try_into().expect("u32 to usize"),
                    name_index: field_name_index.try_into().expect("u32 to usize"),
                });
            }

            final_input = next_input;

            structs.push(DnaStruct {
                type_index: struct_name_index.try_into().expect("u32 to usize"),
                fields,
            });
        }

        Ok((final_input, structs))
    }

    pub fn dna<'a, 'b>(&'a self, input: &'b [u8]) -> Result<'b, Dna>
    where
        'b: 'a,
    {
        let (input, _) = tag("SDNA")(input)?;
        let (input, names) = self.names(input)?;
        let (input, types) = self.types(input)?;
        let (input, structs) = self.structs(input)?;

        Ok((
            input,
            Dna {
                names,
                types,
                structs,
            },
        ))
    }
}
