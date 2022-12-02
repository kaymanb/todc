mod integer;

pub use integer::IntegerRegister;

pub trait Register: Clone {
    type Value;

    fn new(value: Self::Value) -> Self;

    fn read(&self) -> Self::Value;

    fn write(&self, value: Self::Value) -> ();
}

// Cute macro for sharing tests across different implementations of the trait.
// See: https://eli.thegreenplace.net/2021/testing-multiple-implementations-of-a-trait-in-rust/
#[macro_export]
macro_rules! register_tests {
    ($($name:ident: $type:ty,)*) => {
    $(
        mod $name {
            use super::*;
            use crate::register::Register;

			#[test]
			fn test_new() {
				<$type>::new(0);
			}

			#[test]
			fn test_read() {
				let register = <$type>::new(0);
				assert_eq!(0, register.read());
			}

			#[test]
			fn test_write() {
				let register = <$type>::new(0);
				register.write(1);
				assert_eq!(1, register.read());
			}

			#[test]
			fn test_clone() {
				let register = <$type>::new(1);
				assert_eq!(register.read(), register.clone().read());
			}
        }
    )*
    }
}

