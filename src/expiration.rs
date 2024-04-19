use crate::{get_current_time, get_memory_instance};

pub fn ttl() {
  get_memory_instance().remove_expired(get_current_time());
}