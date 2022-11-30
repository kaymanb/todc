use std::sync::Arc;
use std::thread;
use std::time::Duration;
use todc::IntegerRegister;

fn main() {
    let register = Arc::new(IntegerRegister::new(0));
    
    for i in 0..3 {
        let register = Arc::clone(&register);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1));
            register.write(i);
            println!("Process {} read {}", i, register.read());
        });
    }
    
    thread::sleep(Duration::from_secs(1));

    let result = register.read();
    println!("Main thread received: {}", result);
}
