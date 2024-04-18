use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct MemoryStore {
    memory: HashMap<String, String>,
    expire: HashMap<String, u128>
}

impl MemoryStore {
    fn new() -> Self {
        MemoryStore {
            memory: HashMap::new(),
            expire: HashMap::new(),
        }
    }

    fn set(&mut self, key: String, value: String) {
        self.memory.insert(key, value);
    }

    fn get(&mut self, key: &str) -> Option<&String> {
        self.memory.get(key)
    }

    fn expire(&mut self, key: String, ttl: u128) {
        self.expire.insert(key, ttl);
    }

    fn remove_expired(&mut self, current_time: u128) {
        let mut keys_deleted: Vec<String> = Vec::new();
        if self.expire.keys().len() > 0 {
            for (k,ttl) in &self.expire {
                if current_time > *ttl {
                    self.memory.remove(k);
                    keys_deleted.push(k.to_string());
                }
            }
        }
        for key in keys_deleted {
            self.expire.remove(&key);
        }
    }
}

static mut MEMORY_STORE_INSTANCE: Option<MemoryStore> = None;

fn get_memory_instance() -> &'static mut MemoryStore {
    unsafe {
        MEMORY_STORE_INSTANCE.get_or_insert_with(|| MemoryStore::new())
    }
}

fn get_current_time() -> u128 {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u128 * 1000 + since_epoch.subsec_millis() as u128
}

fn ttl_thread() {
    let mut last_run = Instant::now();
    loop {
        let elapsed = last_run.elapsed();
        if elapsed < Duration::from_millis(10) {
            thread::sleep(Duration::from_millis(10) - elapsed);
        }
        get_memory_instance().remove_expired(get_current_time());
        last_run = Instant::now();
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    thread::spawn(|| {
        ttl_thread();
    });

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

fn parse_u8(st: &[u8]) -> u8 {
    let x = &*st;
    let parsed = std::str::from_utf8(x).expect("failed on extract");
    return parsed.parse::<u8>().expect("failed in conversion");
}

fn response_parser(res: String) -> Vec<u8> {
    let formatted_response = format!("+{}\r\n", res);
    formatted_response.as_bytes().to_vec()
}

fn process_commands(commands: Vec<String>) -> Vec<u8> {
    let mut raw_response = "";
    if let Some(first_element) = commands.first() {
        match first_element.as_str() {
            "PING" => { raw_response = "PONG"; },
            "ECHO" => { raw_response = &commands[1]; },
            "GET" => {
                match get_memory_instance().get(&commands[1]) {
                    Some(value) => {
                        raw_response = value;
                    }
                    None => {
                        return b"$-1\r\n".to_vec();
                    }
                }
            }
            "SET" => {
                let memory = get_memory_instance();
                memory.set(commands[1].to_string(), commands[2].to_string());
                memory.expire(commands[1].to_string(), get_current_time() + 10000);
                raw_response = "OK";
            },
            _ => {},
        }
    } else {}
    return response_parser(raw_response.to_string());
}

fn get_header(cursor: usize, buff: [u8; 255]) -> usize {
    let mut n_cursor = 0;
    for h in cursor..buff.len() {
        if buff[h] == 13 && buff[h + 1] == 10 {
            n_cursor = ((h + 1) + 1) - cursor;
            break;
        }
    }
    n_cursor
}

fn parser(buff: [u8; 255]) -> Vec<String> {
    let mut cursor = 0;
    let mut commands: Vec<String> = Vec::new();

    let mut cursor_delta = get_header(cursor, buff);
    let header_buffer = &buff[(cursor + 1)..(cursor + cursor_delta - 2)];
    cursor += cursor_delta;

    let header_size = parse_u8(&header_buffer);

    for _i in 0..header_size {
        cursor_delta = get_header(cursor, buff);
        let data_buffer_header = &buff[(cursor + 1)..(cursor + cursor_delta - 2)];
        cursor += cursor_delta;

        let data_buffer_size: u8 = parse_u8(&data_buffer_header);
        let data_buffer = &buff[cursor..(cursor + data_buffer_size as usize)];
        cursor += data_buffer_size as usize;
        cursor += 2;

        let _c = std::str::from_utf8(&data_buffer).unwrap();
        commands.push(_c.to_string());
    }
    commands[0] = commands[0].to_ascii_uppercase();
    commands
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
