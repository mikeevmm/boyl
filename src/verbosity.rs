use num_traits::PrimInt;

#[derive(Debug)]
pub enum Verbosity {
    None,
    Some,
    Very,
}

impl<X> From<X> for Verbosity
where
    X: PrimInt,
{
    fn from(value: X) -> Self {
        if value.lt(&X::from(1).unwrap()) {
            Verbosity::None
        } else if value.lt(&X::from(2).unwrap()) {
            Verbosity::Some
        } else {
            Verbosity::Very
        }
    }
}
