use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;
use tokio::net::TcpStream;

use crate::get_options_instance;
use crate::get_replicas_instance;
use crate::get_replication_instance;


struct ReplicaInfo {
  // id: Option<String>,
  port: Option<String>,
  // capabilities: Option<String>
}

impl ReplicaInfo {
  fn new() -> Self {
      ReplicaInfo {
          // id: None,
          port: None,
          // capabilities: None,
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
    println!("response: {}", String::from_utf8_lossy(&mut buff).to_string());
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
      // replicas_available:  0,
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

  pub async fn replicate(&mut self, addr: SocketAddr) {
    println!("address:: {}", addr);
    match TcpStream::connect(addr).await {
      Ok(mut replica) => {
        let mut pending_to_delete: Vec<String> = Vec::new();
        for (task_port, _task_data) in get_replication_instance().get() {
          let _ = replica.write_all("*6\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$1\r\n1\r\n$3\r\nSET\r\n$3\r\nbar\r\n$1\r\n2\r\n$3\r\nSET\r\n$3\r\nbaz\r\n$1\r\n3\r\n".as_bytes()).await;
          pending_to_delete.push(task_port.to_string());
          println!("Command successfully propagated!");

        }
        for key in &pending_to_delete {
          get_replication_instance().remove(key);
        }
      }
      Err(err) => {
        eprintln!("Error on socket connection {}", err);
      }
    }
  }
}



