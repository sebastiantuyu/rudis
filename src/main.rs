use std::net::TcpListener;

fn main() {
    println!("Logs from your program will appear here!");
    
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("Accepted new connection");
            }
            Err(e) => {
                eprintln!("Error on connection {}", e);
            }
        }
    }
}
