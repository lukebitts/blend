use nom::{
    alt, call, complete, delimited, do_parse, error_position, many0, many1, map, named, none_of,
    peek, tag, take_until, tuple_parser, value,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FieldInfo {
    Value,
    ValueArray1D { len: usize },
    ValueArray2D { len1: usize, len2: usize },
    Pointer { indirection_count: usize },
    PointerArray1D { len: usize },
    FnPointer,
}

named!(fn_pointer < &str, (&str, FieldInfo) >,
    do_parse!(
        name: delimited!(tag!("(*"), take_until!(")"), tag!(")")) >>
        delimited!(tag!("("), take_until!(")"), tag!(")")) >>
        ((name, FieldInfo::FnPointer))
    )
);

fn property_name(input: &str) -> nom::IResult<&str, &str> {
    if let Some(index) = input.find('[') {
        Ok((&input[index..], &input[0..index]))
    } else {
        Ok(("", input))
    }
}

named!(pointer < &str, (&str, FieldInfo) >,
    do_parse!(
        asterisks: many1!(tag!("*")) >>
        name: property_name >>
        ((name, FieldInfo::Pointer{ indirection_count: asterisks.len() }))
    )
);

named!(value < &str, (&str, FieldInfo) >,
    do_parse!(
        peek!(none_of!("*(")) >>
        name: property_name >>
        ((name, FieldInfo::Value))
    )
);

named!(pub parse_field (&str) -> (&str, FieldInfo),
    do_parse!(
        field_info: alt!(
            value | pointer | fn_pointer
        ) >>
        array_sizes: map!(
            many0!(complete!(delimited!(tag!("["), take_until!("]"), tag!("]")))), 
            |input: Vec<&str>|{
                let mut sizes = Vec::new();
                for size in input {
                    sizes.push(size.parse::<usize>().unwrap());
                }

                sizes
            }
        ) >>
        field: map!(value!((field_info, array_sizes)), |((field_name, field_info), array_sizes)| {
            match field_info {
                FieldInfo::Value => {
                    if array_sizes.is_empty() {
                        (field_name, field_info)
                    }
                    else if array_sizes.len() == 1 {
                        (field_name, FieldInfo::ValueArray1D { len: array_sizes[0] } )
                    }
                    else if array_sizes.len() == 2 {
                        (field_name, FieldInfo::ValueArray2D { len1: array_sizes[0], len2: array_sizes[1] } )
                    }
                    else {
                        panic!("unsuported array dimension {}: {:?}", field_name, array_sizes);
                    }
                }
                FieldInfo::Pointer { .. } => {
                    if array_sizes.is_empty() {
                        (field_name, field_info)
                    }
                    else if array_sizes.len() == 1 {
                        (field_name, FieldInfo::PointerArray1D { len: array_sizes[0] })
                    }
                    else {
                        panic!("unsuported pointer array dimension")
                    }
                }
                FieldInfo::FnPointer { .. } => {
                    assert!(array_sizes.is_empty());
                    (field_name, field_info)
                }
                _ => panic!("invalid field info")
            }
        }) >>
        (field)
    )
);
