use std::collections::HashMap;
pub struct Replication {
  pending: HashMap<String, Vec<u8>>
}

impl Replication {
  pub fn new() -> Self {
    Replication {
      pending: HashMap::new()
    }
  }

  pub fn get_latest_rdb(&mut self) -> Vec<u8> {
    let rdb_as_hex = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";
    let _hex_to_bytes = hex::decode(rdb_as_hex).unwrap();
    let mut r = format!("${}\r\n", _hex_to_bytes.len()).into_bytes();
    r.extend(_hex_to_bytes);
    return r;
  }

  pub fn fullresync(&mut self, to: &str) {
    let data = self.get_latest_rdb();
    self.add_to_queue(to, data);
  }

  pub fn add_to_queue(&mut self, to: &str, data: Vec<u8>) {
    self.pending.insert(to.to_string(), data);
  }

  pub fn get(&mut self) -> &HashMap<String, Vec<u8>> {
    &self.pending
  }

  pub fn remove(&mut self, key: &str) {
    self.pending.remove(key);
  }
}