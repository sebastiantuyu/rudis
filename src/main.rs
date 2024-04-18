use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    for listener_stream in listener.incoming() {
        match listener_stream {
            Ok(stream) => {
                println!("Incoming request: {:?}", stream.peer_addr().unwrap());
                thread::spawn(move ||  {
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
            _ => b"",
        }
    } else {
        b""
    }
}

fn handler(mut stream: TcpStream) {
    let mut commands: Vec<String> = Vec::new();
    let mut headerbuffer = [0; 4];
    stream.read(&mut headerbuffer).expect("Failed");
    let headersize = parse_u8(&headerbuffer[1]);

    for _i in 0..headersize {
        let mut databufferheader = [0; 4];
        stream.read(&mut databufferheader).expect("failed");

        let databuffersize = parse_u8(&databufferheader[1]);
        let mut databuffer: Vec<u8> = vec![0; databuffersize.into()];
        stream.read(&mut databuffer).expect("failed");
        let mut trash = [0;2];
        stream.read(&mut trash).expect("failed");

        let _c = std::str::from_utf8(&databuffer).unwrap();
        commands.push(_c.to_string().to_ascii_uppercase());
    }

    let res = process_commands(commands);

    stream.write(res).expect("failed!");
    stream.flush().expect("failed");
    return ();
}
