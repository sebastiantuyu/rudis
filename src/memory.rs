use std::collections::HashMap;

pub struct MemoryStore {
  memory: HashMap<String, String>,
  expire: HashMap<String, u128>
}


impl MemoryStore {
  pub fn new() -> Self {
      MemoryStore {
          memory: HashMap::new(),
          expire: HashMap::new(),
      }
  }

  pub fn set(&mut self, key: String, value: String) {
      self.memory.insert(key, value);
  }

  pub fn get(&mut self, key: &str) -> Option<&String> {
      self.memory.get(key)
  }

  pub fn expire(&mut self, key: String, ttl: u128) {
      self.expire.insert(key, ttl);
  }

  pub fn remove_expired(&mut self, current_time: u128) {
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