mod replica;
mod expiration;
mod parser;
mod commands;
mod options;
mod memory;

use commands::process_commands;
use memory::MemoryStore;
use options::Options;
use parser::parser;
use replica::handle_replica;
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::expiration::ttl_thread;
use crate::options::read_options;


static mut MEMORY_STORE_INSTANCE: Option<MemoryStore> = None;
static mut OPTIONS: Option<Options> = None;

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

fn main() {
    read_options();
    let port = get_options_instance().get("port").unwrap();
    let replica = get_options_instance().get("role").unwrap();

    let listener = TcpListener::bind(
        format!("127.0.0.1:{}", port)
    ).unwrap();


    thread::spawn(|| {
        ttl_thread();
    });

    if replica == "slave" {
        thread::spawn(|| {
            handle_replica();
        });
    }

    println!("[Rudis]: Server started on port {}", port);

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


fn handler(mut stream: TcpStream) {
    let mut buff = [0; 255];
    while match stream.read(&mut buff) {
        Ok(size) if size > 0 => {
            let commands = parser(buff);
            let res = process_commands(commands);
            stream.write_all(&res).expect("Fail");
            true
        }
        _ => {
            println!("Client disconnected");
            false
        }
    } {}
}
