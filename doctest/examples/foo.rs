use doctest::{foo, quux};

fn main() {
    assert_eq!(foo() + quux(), 4);
}
