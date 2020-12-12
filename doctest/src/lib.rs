pub mod other_mod;
mod private;

pub use private::quux;

pub fn foo() -> i32 {
    1
}

pub struct A;

impl A {
    pub fn b(&self) -> i32 { 0 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
