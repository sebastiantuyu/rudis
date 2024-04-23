
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{get_current_time, get_memory_instance, get_options_instance, get_replicas_instance, get_replication_instance, Connection, Queue, ReplicasList};

fn response_parser(res: String) -> Vec<u8> {
    let formatted_response = format!("+{}\r\n", res);
    formatted_response.as_bytes().to_vec()
}

pub async fn process_commands(commands: Vec<String>, buff: Vec<u8>, stream: &Connection, replicas_list: &Arc<Mutex<ReplicasList>>, replication_queue: &Arc<Mutex<Queue>>, replica_status: &mut bool) -> (Vec<Vec<u8>>, bool) {
    let mut raw_response = "";
    if let Some(first_element) = commands.first() {
        match first_element.as_str() {
            "PING" => {
                raw_response = "PONG";
            }
            "ECHO" => {
                raw_response = &commands[1];
            }
            "GET" => match get_memory_instance().get(&commands[1]) {
                Some(value) => {
                    raw_response = value;
                }
                None => {
                    return (vec![b"$-1\r\n".to_vec()], false);
                }
            },
            "SET" => {
                let memory = get_memory_instance();
                memory.set(commands[1].to_string(), commands[2].to_string());

                if commands.len() > 3 {
                    match commands[3].to_ascii_uppercase().as_str() {
                        "PX" => {
                            let ttl = commands[4].parse::<u128>().unwrap();
                            memory.expire(commands[1].to_string(), get_current_time() + ttl);
                        }
                        _ => {}
                    }
                }

                let replicas = &replicas_list.lock().await;

                println!("# replicas: {}", replicas.handles.lock().await.len());
                if replicas.handles.lock().await.len() > 0 {
                    replication_queue.lock().await.tasks.push(buff.to_vec());
                    for replica in replicas.handles.lock().await.iter() {
                        println!("send task to debug");
                        _  = replica.sender.send(crate::ReplicaCommand { message: buff.to_vec() }).await;
                    }
                }
                raw_response = "OK";
            }
            "INFO" => {
                let master_replid = get_options_instance().get("master_replid").unwrap();
                let port = get_options_instance().get("role").unwrap();
                let master_repl_offset = get_options_instance().get("master_repl_offset").unwrap();

                let response = format!(
                    "role:{port}\n\rmaster_replid:{master_replid}\n\rmaster_repl_offset:{master_repl_offset}\n\r"
                );
                return (vec![format!("${}\r\n{response}\r\n", response.len())
                    .as_bytes()
                    .to_vec()], false);
            }
            "REPLCONF" => {
                match commands[1].as_str() {
                    "listening-port" => {
                        get_replicas_instance().add_replica(&commands[2]);
                    }
                    "capa" => {}
                    _ => {}
                }
                raw_response = "OK";
            }
            "PSYNC" => {
                let idl = get_options_instance().get("master_replid").unwrap();
                replicas_list.lock().await.add(stream.stream.peer_addr().unwrap());
                *replica_status = true;

                return (vec![
                    response_parser(format!("FULLRESYNC {} 0", idl)),
                    get_replication_instance().get_latest_rdb(),
                ], false);
            }
            _ => {
                println!("Unrecognized command {:?}", commands);
            }
        }
    } else {
    }
    return (vec![response_parser(raw_response.to_string())], false);
}
