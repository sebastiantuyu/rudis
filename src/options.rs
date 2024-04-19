use std::{collections::HashMap, env::args_os};

use crate::get_options_instance;

pub struct Options  {
  options: HashMap<String, String>,
}

impl Options {
  pub fn new() -> Self {
      Options {
          options: HashMap::new()
      }
  }

  pub fn get(&mut self, key: &str) -> Option<&String> {
    self.options.get(key)
  }

  pub fn set(&mut self, key: &str, value: &str) {
    self.options.insert(key.to_string(), value.to_string());
  }
}


fn load_basic_options() {
  get_options_instance().set("role", "master");
  get_options_instance().set("port", "6379");
  get_options_instance().set("master_repl_offset", "0");
  get_options_instance().set("master_replid", "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb");
}

pub fn read_options() {
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
                      .set("port", &args[2]);
              }
              "replicaof" => {
                  if sz <= 3 {
                      panic!("Missing arguments for [replicaof]");
                  }
                  get_options_instance()
                      .set(
                          "role",
                          "slave"
                      );
                  get_options_instance()
                      .set(
                        "master-host",
                        &args[idx + 1]
                      );
                  get_options_instance()
                      .set(
                        "master-port",
                        &args[idx + 2]
                      );
                  println!("[Rudis]: Initialize as replica mode");
              }
              _ => {}
          }
      }
  }
}

