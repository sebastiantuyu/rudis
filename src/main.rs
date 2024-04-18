use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::env::args_os;

static mut MEMORY_STORE_INSTANCE: Option<MemoryStore> = None;
static mut OPTIONS: Option<Options> = None;

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


struct Options  {
    options: HashMap<String, String>
}

impl Options {
    fn new() -> Self {
        Options {
            options: HashMap::new(),
        }
    }
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

fn get_current_time() -> u128 {
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_secs() as u128 * 1000 + since_epoch.subsec_millis() as u128
}

fn ttl_thread() {
    let mut last_run = Instant::now();
    loop {
        let elapsed = last_run.elapsed();
        if elapsed < Duration::from_millis(1) {
            thread::sleep(Duration::from_millis(1) - elapsed);
        }
        get_memory_instance().remove_expired(get_current_time());
        last_run = Instant::now();
    }
}

fn load_basic_options() {
    let options = get_options_instance();
    options.options.insert("role".to_string(), "master".to_string());
    options.options.insert("port".to_string(), "6379".to_string());
    options.options.insert("master_repl_offset".to_string(), "0".to_string());
    options.options.insert("master_replid".to_string(), "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string());
}

fn read_options() {
    load_basic_options();
    let args: Vec<String> = args_os()
        .map(|arg| arg.into_string().unwrap_or_else(|os_string| {
            os_string.to_string_lossy().to_string()
        }))
        .collect();
    let sz = args.len();

    for (idx, argument) in args.iter().enumerate() {
        let arg = argument;
        if arg.starts_with("--") {
            let option: Vec<String> = arg.split("--").map(|v| v.to_string()).collect();
            match option[1].as_str() {
                "port" => {
                    if sz <= 2 {
                        panic!("Missing arguments for [port]");
                    }
                    get_options_instance()
                        .options
                        .insert("port".to_string(), args[2].to_string());
                }
                "replicaof" => {
                    if sz <= 3 {
                        panic!("Missing arguments for [replicaof]");
                    }
                    get_options_instance()
                        .options
                        .insert(
                            "role".to_string(),
                            "slave".to_string()
                        );
                    get_options_instance()
                        .options
                        .insert(
                            "master-host".to_string(),
                            args[idx + 1].to_string()
                        );
                    get_options_instance()
                        .options
                        .insert(
                            "master-port".to_string(),
                            args[idx + 2].to_string()
                        );
                }
                _ => {}
            }
        }
    }
}

fn handle_replica() {
    // match TcpStream::connect("127.0.0.1:6379") {
    //     Ok(stream) => {
    //         print!("connected to master properly : ) ");
    //     }
    //     Err(err) => eprintln!("Failed to connect into master {}", err)
    // }
}

fn main() {
    read_options();
    let port = get_options_instance().options.get("port").unwrap();
    let replica = get_options_instance().options.get("role").unwrap();

    let listener = TcpListener::bind(
        format!("127.0.0.1:{}", port)
    ).unwrap();


    thread::spawn(|| {
        ttl_thread();
    });

    if replica == "replica" {
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

                if commands.len() > 3 {
                    match commands[3].to_ascii_uppercase().as_str() {
                        "PX" => {
                            let ttl = commands[4].parse::<u128>().unwrap();
                            memory.expire(commands[1].to_string(), get_current_time() + ttl);
                        },
                        _ => {}
                    }
                }
                raw_response = "OK";
            },
            "INFO" => {
                let options = &get_options_instance().options;
                let master_replid = options.get("master_replid").unwrap();
                let port = options.get("role").unwrap();
                let master_repl_offset = options.get("master_repl_offset").unwrap();

                let response = format!(
                    "role:{port}\n\rmaster_replid:{master_replid}\n\rmaster_repl_offset:{master_repl_offset}\n\r"
                );
                return format!("${}\r\n{response}\r\n", response.len()).as_bytes().to_vec();
            }
            _ => {},
        }
    } else {}
    return response_parser(raw_response.to_string());
}

fn get_header(cursor: &usize, buff: [u8; 255]) -> (usize, u8) {
    let mut delta = 0;
    for h in *cursor..buff.len() {
        if buff[h] == 13 && buff[h + 1] == 10 {
            delta = ((h + 1) + 1) - cursor;
            break;
        }
    }
    let header_buffer = &buff[(cursor + 1)..(cursor + delta - 2)];
    return (delta, parse_u8(header_buffer))
}

fn parser(buff: [u8; 255]) -> Vec<String> {
    let mut cursor = 0;
    let mut commands: Vec<String> = Vec::new();

    let (cursor_delta, header_size) = get_header(&cursor, buff);
    cursor += cursor_delta;

    for _i in 0..header_size {
        let (cursor_delta, data_buffer_size) = get_header(&cursor, buff);
        cursor += cursor_delta;

        let data_buffer = &buff[cursor..(cursor + data_buffer_size as usize)];
        cursor += data_buffer_size as usize + 2;

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
