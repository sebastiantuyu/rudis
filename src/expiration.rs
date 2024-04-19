
use std::thread;
use std::time::{Duration, Instant};

use crate::{get_current_time, get_memory_instance};

pub fn ttl_thread() {
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