use std::sync::Arc;
use std::thread;
//use std::time::Duration;
use todc::register::{AtomicRegister, Register};

fn main() {
    let num_threads = 3;
    let registers = Arc::new(vec![AtomicRegister::new(0); num_threads]);

    for i in 0..num_threads {
        let registers = Arc::clone(&registers);
        thread::spawn(move || {
            registers[i].write(1);
        });
    }

    for i in 0..num_threads {
        println!("Read {} from register {}", registers[i].read(), i);
    }
}
