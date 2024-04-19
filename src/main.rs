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
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::expiration::ttl;
use crate::options::read_options;
use crate::replica::sync_to_master;
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

fn main() {
    read_options();
    let port = get_options_instance().get("port").unwrap();
    let listener = TcpListener::bind(
        format!("0.0.0.0:{}", port)
    ).unwrap();

    thread::spawn(|| {
        let mut last_run = Instant::now();
        loop {
            let elapsed = last_run.elapsed();
            if elapsed < Duration::from_millis(1) {
                thread::sleep(Duration::from_millis(1) - elapsed);
            }

            ttl();
            match  get_options_instance().get("role").unwrap().as_str() {
                "slave" => {
                    if !get_replicas_instance().status() { sync_to_master(); }
                }
                _ => {}
            }
            last_run = Instant::now();
        }
    });

    println!("[Rudis]: Server started on port {}", port);
    for listener_stream in listener.incoming() {
        match listener_stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handler(stream);
                });
            }
            Err(e) => {
                println!("Error on connection {}", e);
            }
        }
    }
}


fn handler(mut stream: TcpStream) {
    let mut buff = [0; 255];
    println!("New connection: {}", stream.peer_addr().unwrap());

    loop {
        match stream.read(&mut buff) {
            Ok(size) if size > 0 => {
                if &buff[(size - 2)..size] == [13, 10] {
                    let commands = parser(buff);
                    let responses = process_commands(commands);
                    for response in &responses {
                        stream.write(response).expect("Fail");
                    }
                } else {
                    println!("[Rudis]: Syncing data");
                    stream.write(&"+OK\r\n".as_bytes()).expect("failed");
                }
            }
            _ => {}
        }
    }
}
