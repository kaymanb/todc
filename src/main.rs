use std::sync::Arc;
use std::thread;
//use std::time::Duration;
use todc::{IntegerRegister, Register};

fn main() {
    let num_threads = 3;
    let registers: Arc<Vec<IntegerRegister>> = Arc::new(vec![Register::new(0); num_threads]);
     
    for i in 0..num_threads {
        let registers = Arc::clone(&registers);
        thread::spawn(move || {
            registers[i].write(1);
        });
    }

    for i in 0..num_threads {
        println!("Read {} from register {}", registers[i].read(), i);
    }
    //for i in 0..3 {
    //let register: Arc<IntegerRegister> = Arc::new(Register::new(0));
    //    let register = Arc::clone(&register);
    //    thread::spawn(move || {
    //        thread::sleep(Duration::from_millis(1));
    //        register.write(i);
    //        println!("Process {} read {}", i, register.read());
    //    });
    //}
    //
    //thread::sleep(Duration::from_secs(1));

    //let result = register.read();
    //println!("Main thread received: {}", result);
}
