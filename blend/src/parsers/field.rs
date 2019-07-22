use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until},
    combinator::complete,
    error::{ErrorKind, ParseError},
    multi::{many0, many1},
    sequence::delimited,
    Err, IResult,
};

#[derive(Debug)]
pub enum FieldParseError {
    NomError {
        kind: ErrorKind,
        other: Option<Box<FieldParseError>>,
    },
    InvalidArraySize,
}

impl ParseError<&str> for FieldParseError {
    fn from_error_kind(_input: &str, kind: ErrorKind) -> Self {
        FieldParseError::NomError { kind, other: None }
    }

    fn append(_input: &str, kind: ErrorKind, other: Self) -> Self {
        FieldParseError::NomError {
            kind,
            other: Some(Box::new(other)),
        }
    }
}

type Result<'a, T> = IResult<&'a str, T, FieldParseError>;

#[derive(Debug, Clone)]
pub enum FieldInfo {
    Value,
    ValueArray {
        len: usize,
        //todo: rename to dimensions
        dimensions_len: Vec<usize>,
    },
    Pointer {
        indirection_count: usize,
    },
    PointerArray {
        indirection_count: usize,
        len: usize,
        //todo: rename to dimensions
        dimensions_len: Vec<usize>,
    },
    FnPointer,
}

pub fn fn_pointer(input: &str) -> Result<(&str, FieldInfo)> {
    let (input, name) = delimited(tag("(*"), take_until(")"), tag(")"))(input)?;

    let (input, _) = delimited(tag("("), take_until(")"), tag(")"))(input)?;

    Ok((input, (name, FieldInfo::FnPointer)))
}

fn array_dimensions(input: &str) -> Result<Vec<usize>> {
    let (input, array_dimensions) =
        many0(complete(delimited(tag("["), take_until("]"), tag("]"))))(input)?;

    let mut dimensions_len = Vec::new();
    for dimension_str in array_dimensions {
        dimensions_len.push(
            dimension_str
                .parse::<usize>()
                .map_err(|_| Err::Failure(FieldParseError::InvalidArraySize))?,
        );
    }

    Ok((input, dimensions_len))
}

fn pointer(input: &str) -> Result<(&str, FieldInfo)> {
    let (input, asterisks) = many1(tag("*"))(input)?;
    let (input, name) = take_till(|c| c == '[')(input)?;

    if !input.is_empty() {
        let (input, dimensions_len) = array_dimensions(input)?;
        let len = dimensions_len.iter().product();
        Ok((
            input,
            (
                name,
                FieldInfo::PointerArray {
                    indirection_count: asterisks.len(),
                    len,
                    dimensions_len,
                },
            ),
        ))
    } else {
        Ok((
            input,
            (
                name,
                FieldInfo::Pointer {
                    indirection_count: asterisks.len(),
                },
            ),
        ))
    }
}

fn value(input: &str) -> Result<(&str, FieldInfo)> {
    let (input, name) = take_till(|c| c == '[')(input)?;
    if !input.is_empty() {
        let (input, dimensions_len) = array_dimensions(input)?;
        let len = dimensions_len.iter().product();
        Ok((
            input,
            (
                name,
                FieldInfo::ValueArray {
                    len,
                    dimensions_len,
                },
            ),
        ))
    } else {
        Ok((input, (name, FieldInfo::Value)))
    }
}

pub fn parse_field(input: &str) -> Result<(&str, FieldInfo)> {
    alt((fn_pointer, pointer, value))(input)
}
