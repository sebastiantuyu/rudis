use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    println!("[Rudis]: Server started on port 6379");
    for listener_stream in listener.incoming() {
        match listener_stream {
            Ok(stream) => {
                println!("Incoming request: {:?}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    handler(stream);
                });
            }
            Err(e) => {
                println!("Error on connection {}", e);
            }
        }
    }
}

fn parse_u8(st: &u8) -> u8 {
    let x = &[*st];
    let parsed = std::str::from_utf8(x).expect("failed on extract");
    return parsed.parse::<u8>().expect("failed in conversion");
}

fn process_commands(commands: Vec<String>) -> &'static [u8] {
    if let Some(first_element) = commands.first() {
        match first_element.as_str() {
            "PING" => b"+PONG\r\n",
            _ => b"+\r\n",
        }
    } else {
        b""
    }
}

fn handler(mut stream: TcpStream) {
    let mut buff = [0; 512];
    let mut cursor = 0;
    while match stream.read(&mut buff) {
        Ok(size) if size > 0 => {
            let mut commands: Vec<String> = Vec::new();
            let header_buffer = &buff[cursor..(cursor + 4)];
            cursor += 4;
            let header_size = parse_u8(&header_buffer[1]);

            for _i in 0..header_size {
                let data_buffer_header = &buff[cursor..(cursor +4)];
                cursor += 4;

                let data_buffer_size: u8 = parse_u8(&data_buffer_header[1]);
                let data_buffer = &buff[cursor..(cursor + data_buffer_size as usize)];
                cursor += data_buffer_size as usize;
                cursor += 2;

                let _c = std::str::from_utf8(&data_buffer).unwrap();
                commands.push(_c.to_string().to_ascii_uppercase());
            }
            cursor = 0;
            let res = process_commands(commands);
            stream.write_all(res).expect("Fail");
            true
        }
        _ => {
            println!("Client disconnected");
            false
        }
    } {}
}
