// use std::collections::HashMap;
use std::net::TcpStream;
use std::io::Write;
use std::io::Read;

use crate::get_options_instance;

// static mut REPLICAS: Option<Replicas> = None;

// struct ReplicaInfo {
//   id: Option<String>,
//   port: Option<String>,
//   capabilities: Option<String>
// }

// impl ReplicaInfo {
//   fn new() -> Self {
//       ReplicaInfo {
//           id: None,
//           port: None,
//           capabilities: None,
//       }
//   }
// }

// pub struct Replicas {
//   replicas: HashMap<String, ReplicaInfo>
// }

// impl Replicas {
//   pub fn new() -> Self {
//     Replicas {
//       replicas: HashMap::new(),
//     }
//   }
// }


fn parse_str_to_repl(data: Vec<&str>) -> Vec<u8> {
  let mut repl = format!("*{}\r\n", data.len());
  for d in data {
    repl +=  &format!("${}\r\n{}\r\n", d.len(), d);
  }
  repl.as_bytes().to_owned()
}

fn send_and_response(stream: &mut TcpStream, data: Vec<&str>) {
  match stream.write_all(&parse_str_to_repl(data)) {
      Ok(_m) =>  println!("Success sending to master"),
      Err(_err) =>  eprintln!("error while connecting to master"),
  }

  let mut buffer = [0; 128];
  let bytes_read = stream.read(&mut buffer).expect("failed reading buffer");
  let response = String::from_utf8_lossy(&buffer[..bytes_read]);
  println!("Server response: {}", response);
}

pub fn handle_replica() {
  let master_port = get_options_instance().get("master-port").unwrap();
  let port = get_options_instance().get("port").unwrap();

  match TcpStream::connect(format!("127.0.0.1:{}", master_port)) {
      Ok(mut stream) => {
          println!("[Rudis]: Connected to master properly on: {}", master_port);
          send_and_response(&mut stream, vec! ["PING"]);
          send_and_response(&mut stream, vec! ["REPLCONF", "listening-port", port]);
          send_and_response(&mut stream, vec! ["REPLCONF", "capa", "psync2"]);
      },
      Err(err) => eprintln!("Failed to connect into master {}", err)
  }
}