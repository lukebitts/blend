use crate::field_parser::FieldInfo;

#[derive(Debug)]
pub struct FieldTemplate {
    //pub name: String,
    pub info: FieldInfo,
    pub type_index: u16,
    pub type_name: String,
    pub data_start: usize,
    pub data_len: usize,
    pub is_primitive: bool,
}
