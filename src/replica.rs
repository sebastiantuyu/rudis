use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::get_options_instance;

struct ReplicaInfo {
  port: Option<String>,
}

impl ReplicaInfo {
  fn new() -> Self {
      ReplicaInfo {
          port: None,
      }
  }

  fn set_port(&mut self, port: &str) {
    self.port = Some(port.to_string());
  }
}


fn parse_str_to_repl(data: Vec<&str>) -> Vec<u8> {
  let mut repl = format!("*{}\r\n", data.len());
  for d in data {
    repl +=  &format!("${}\r\n{}\r\n", d.len(), d);
  }
  repl.as_bytes().to_owned()
}

async fn send_and_response(stream: &mut TcpStream, data: Vec<&str>) -> Option<String> {
  println!("{:?}", data);
  let _ = stream.write_all(&parse_str_to_repl(data)).await;
  _ = stream.flush().await;

  let mut buff = [0; 255];
  let size = stream.read(&mut buff).await.unwrap();
  if size > 0 {
    println!("response: {}", String::from_utf8_lossy(&mut buff).to_string().trim_end());
    println!("end-response");
  }

  None
}

pub struct Replicas {
  replicas: HashMap<String, ReplicaInfo>,
  has_setup: bool,
  replicas_conn: Vec<String>,
  replicas_list: Vec<TcpStream>
}

impl Replicas {
  pub fn new() -> Self {
    Replicas {
      replicas: HashMap::new(),
      has_setup: false,
      replicas_conn: Vec::new(),
      replicas_list: Vec::new()
    }
  }

  pub fn status(&mut self) -> bool {
    self.has_setup
  }

  pub fn set_status(&mut self, status: bool) {
    self.has_setup = status;
  }

  pub fn available(&mut self) -> usize {
    return self.replicas_conn.len()
  }

  pub fn add_replica(&mut self, port_id: &str) {
    let mut replica_data = ReplicaInfo::new();
    replica_data.set_port(&port_id);

    self.replicas.insert(port_id.to_string(), replica_data);
    self.replicas_conn.push(port_id.to_string());
  }

  pub fn latest(&mut self) -> Option<&String> {
    self.replicas_conn.last()
  }

  pub fn add_replica_stream(&mut self, stream: TcpStream) {
    self.replicas_list.push(stream);
    println!("{}", self.replicas_list.len());
  }

  pub async fn sync_to_master(&mut self) -> TcpStream {
    println!("[Redis][replica]: Attempting syncing with master");
    let master_port = get_options_instance().get("master-port").unwrap();
    let port = get_options_instance().get("port").unwrap();

    let mut listener = TcpStream::connect(format!("0.0.0.0:{}", master_port)).await.expect("failed");

    {
      _ = send_and_response(&mut listener, vec! ["PING"]).await;
    }
    {
      _ = send_and_response(&mut listener, vec! ["REPLCONF", "listening-port", port]).await;
    }
    {
      _ = send_and_response(&mut listener, vec! ["REPLCONF", "capa", "psync2"]).await;
    }
    {
      _ = send_and_response(&mut listener, vec! ["PSYNC", "?", "-1"]).await;
    }
    listener
  }
}



