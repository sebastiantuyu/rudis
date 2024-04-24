// 36 ===> $
// 42 ===> *
// 43 ===> +

fn parse_u8(st: &[u8]) -> u8 {
  let x = &*st;
  let parsed = std::str::from_utf8(x).expect("failed on extract");
  return parsed.parse::<u8>().expect("failed in conversion");
}


fn get_header(cursor: &mut usize, buff: Vec<u8>) -> (usize, u8) {
  let mut delta = 0;
  let mut founded = 0;

  for h in *cursor..buff.len() {
    if buff[h] == 13 && buff[h + 1] == 10 {
      delta = ((h + 1) + 1) - *cursor;
      founded += 1;
      break;
    }
  }

  if founded == 0 {
    delta = buff.len() - *cursor;
  }

  let header_buffer = &buff[(*cursor + 1)..(*cursor + delta - 2)];
  return (delta, parse_u8(header_buffer))
}

pub fn parser(buff: [u8; 255]) -> Vec<String> {
  let mut cursor: usize = 0;
  let mut commands: Vec<String> = Vec::new();

  let (cursor_delta, header_size) = get_header(&mut cursor, buff.to_vec());
  cursor += cursor_delta;

  for _i in 0..header_size {
    let (cursor_delta, data_buffer_size) = get_header(&mut cursor, buff.to_vec());
    cursor += cursor_delta;

    let data_buffer = &buff[cursor..(cursor + data_buffer_size as usize)];
    cursor += data_buffer_size as usize + 2;

    let _c = std::str::from_utf8(&data_buffer).unwrap();
    commands.push(_c.to_string());
  }
  commands[0] = commands[0].to_ascii_uppercase();
  commands
}


pub fn parser_v3(buff: &Vec<u8>) -> Vec<String> {
  let mut cursor: usize = 0;
  let mut commands: Vec<String> = Vec::new();

  let (cursor_delta, header_size) = get_header(&mut cursor, buff.to_vec());
  cursor += cursor_delta;

  for _i in 0..header_size {
    let (cursor_delta, data_buffer_size) = get_header(&mut cursor, buff.to_vec());
    cursor += cursor_delta;

    let data_buffer = &buff[cursor..(cursor + data_buffer_size as usize)];
    cursor += data_buffer_size as usize + 2;

    let _c = std::str::from_utf8(&data_buffer).unwrap();
    commands.push(_c.to_string());
  }
  commands[0] = commands[0].to_ascii_uppercase();
  commands
}

pub fn parser_v2(bytes: [u8; 255]) -> Vec<Vec<String>> {
  let mut sections: Vec<Vec<u8>> = Vec::new();
  let mut section_commands: Vec<Vec<String>> = Vec::new();
  let mut start_index: usize = 0;

  for (i, &byte) in bytes.iter().enumerate() {
      if byte == 42 || byte == 43 || byte == 0 {
          if start_index != i {
              let sub_vec = bytes[(start_index)..i].to_vec();
              sections.push(sub_vec);
          }
          if byte == 0 {
              break;
          }
          start_index = i;
      }
  }

  if start_index < bytes.len() {
    let sub_vector = bytes[start_index..].to_vec();
    sections.push(sub_vector);
  }

  for section in &sections {
    if section.len() <= 60 {
      section_commands.push(parser_v3(section));
    }
  }

  section_commands
}