mod integer;

pub use integer::IntegerRegister;

pub trait Register: Clone {
    type Value;

    fn new(value: Self::Value) -> Self;

    fn read(&self) -> Self::Value;

    fn write(&self, value: Self::Value) -> ();
}

