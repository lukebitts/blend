mod blend_to_string;
pub mod parsers;
pub mod runtime;

pub use runtime::{Blend, Instance};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
