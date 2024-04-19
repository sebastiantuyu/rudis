
fn parse_u8(st: &[u8]) -> u8 {
  let x = &*st;
  let parsed = std::str::from_utf8(x).expect("failed on extract");
  return parsed.parse::<u8>().expect("failed in conversion");
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

pub fn parser(buff: [u8; 255]) -> Vec<String> {
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