use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Duration,
};

fn main() {
    println!("Hello, world!");
    loop {
        // crimes here, don't look
        let _ = std::panic::catch_unwind(|| {
            let listener = TcpListener::bind("0.0.0.0:8001").unwrap();

            while let Some((mut stream, _remote)) = listener.accept().ok() {
                println!("connected");
                let mut read_buffer = vec![0u8; 1000];
                loop {
                    if let Ok(read) = stream.read(&mut read_buffer) {
                        println!("read some data: {read} bytes {:?}", &read_buffer[..read]);
                    }
                    stream.write_all(b"hello").unwrap();
                    println!("written");
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }
        });
        println!("panicked, restarting.");
    }
}
