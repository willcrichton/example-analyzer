use doctest::{foo, quux, A};

fn main() {
    assert_eq!(foo() + quux(), 4);
    assert_eq!(A.b(), 0);
}
