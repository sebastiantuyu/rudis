#[allow(dead_code)]
mod replica;
mod expiration;
mod parser;
mod commands;
mod options;
mod memory;
mod replication;

use commands::process_commands;
use memory::MemoryStore;
use options::Options;
use parser::parser;
use replica::Replicas;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::expiration::ttl;
use crate::options::read_options;
use crate::replication::Replication;


static mut MEMORY_STORE_INSTANCE: Option<MemoryStore> = None;
static mut OPTIONS: Option<Options> = None;
static mut REPLICAS: Option<Replicas> = None;
static mut REPLICATION: Option<Replication> = None;

fn get_current_time() -> u128 {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u128 * 1000 + since_epoch.subsec_millis() as u128
}

fn get_options_instance() ->  &'static mut Options {
    unsafe {
        OPTIONS.get_or_insert_with(|| Options::new())
    }
}

fn get_memory_instance() -> &'static mut MemoryStore {
    unsafe {
        MEMORY_STORE_INSTANCE.get_or_insert_with(|| MemoryStore::new())
    }
}

pub fn get_replicas_instance() -> &'static mut Replicas {
    unsafe {
        REPLICAS.get_or_insert_with(|| Replicas::new())
    }
}

pub fn get_replication_instance() -> &'static mut Replication {
    unsafe {
        REPLICATION.get_or_insert_with(|| Replication::new())
    }
}

struct ReplicasList {
    list: Vec<SocketAddr>,
    handles: Mutex<Vec<ReplicaHandle>>
}

struct ReplicaHandle {
    pub sender: Sender<ReplicaCommand>,
}

impl ReplicasList {
    pub fn new() -> Self {
        ReplicasList {
            list: Vec::new(),
            handles: Mutex::new(Vec::new())
        }
    }

    pub fn add(&mut self, addr: SocketAddr) {
        println!("Adding new replica {}", addr.to_string());
        self.list.push(addr);
    }

}

struct Connection {
    pub stream: TcpStream
}

impl Connection {
    pub fn bind(stream: TcpStream) -> Self {
        Self {
            stream
        }
    }

    pub async fn write(&mut self, data: Vec<u8>) -> Result<(), std::io::Error> {
        _ = self.stream.write_all(&data).await;
        self.stream.flush().await
    }

    pub async fn read(&mut self) -> (usize, [u8; 255]){
        let mut buff = [0;255];
        (self.stream.read(&mut buff).await.unwrap(), buff)
    }
}

fn find_last_zero(buff: [u8; 255]) -> i32 {
    let mut index = 0;
    for buf_idx in buff {
        if buf_idx == 0 { break; }
        index += 1;
    }
    index
}

#[tokio::main]
async fn main() {
    read_options();

    thread::spawn(|| {
        let mut last_run = Instant::now();
        loop {
            let elapsed = last_run.elapsed();
            if elapsed < Duration::from_millis(1) {
                thread::sleep(Duration::from_millis(1) - elapsed);
            }
            last_run = Instant::now();
            ttl();
        }
    });
    let replicas: Arc<Mutex<ReplicasList>> = Arc::new(Mutex::new(ReplicasList::new()));
    let port = get_options_instance().get("port").unwrap();

    let listener = TcpListener::bind(
        format!("0.0.0.0:{}", port)
    ).await.unwrap();

    println!("[Rudis]: Server started on port {}", port);

    match  get_options_instance().get("role").unwrap().as_str() {
        "slave" => {
            tokio::spawn(async move {
                let mut connection = get_replicas_instance().sync_to_master().await;
                loop {
                    let mut replication_buff = [0; 255];
                    connection.read(&mut replication_buff).await.unwrap();
                    let last_zero = find_last_zero(replication_buff) as usize;
                        if last_zero > 0  {
                            if &replication_buff[(last_zero - 2)..last_zero] == [13, 10] {
                                let commands = parser(replication_buff);
                                println!("{:?}", commands);
                                if commands[0] == "SET" {
                                    let memory = get_memory_instance();
                                    memory.set(commands[1].to_string(), commands[2].to_string());
                                }

                            } else {}
                        }
                }
            });
        }
        _ => {}
    }

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let replicas_list = replicas.clone();
        let connection = Connection::bind(stream);

        tokio::spawn(async move {
            let _ = handle(connection, replicas_list).await;
        });
    }
}

pub struct ReplicaCommand {
    pub message: Vec<u8>,
}

impl ReplicaCommand {
    pub fn new(message: Vec<u8>) -> Self {
        Self { message }
    }
}

async fn process_sync(mut connection: Connection) -> (ReplicaHandle, JoinHandle<()>){
    let (tx, mut rx) = mpsc::channel::<ReplicaCommand>(32);

    let handle = tokio::spawn(async move {
        loop {
            while let Some(replica_command) = rx.recv().await {
                _ = connection.write(replica_command.message).await;
            }
        }
    });
    (
        ReplicaHandle {
            sender: tx
        },
        handle
    )
}


async fn handle(
    mut connection: Connection,
    replicas: Arc<Mutex<ReplicasList>>
) {
    let mut is_replica = false;

    loop {
        if is_replica {
            let (replica_handle, handle) = process_sync(connection).await;
            replicas.lock().await.handles.lock().await.push(replica_handle);
            _ = handle.await;
            return;
        }
        let (size, buff) = connection.read().await;
        if size > 0 {
            if &buff[(size - 2)..size] == [13, 10] {
                let commands = parser(buff);
                let (responses, _) = process_commands(
                    commands,
                    buff[..size].to_vec(),
                    &connection,
                    &replicas,
                    &mut is_replica
                ).await;
                for response in &responses {
                    _ = connection.write(response.to_vec()).await;
                }
            } else {
                println!("[Rudis]: Syncing data");
                println!("[Rudis][debug]: {}", String::from_utf8_lossy(&buff[..size]));
            }
        }
    }
}