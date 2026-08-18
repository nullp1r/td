pub const URL_SAFE: u8 = 1 << 0;
pub const NO_PADDING: u8 = 1 << 1;

pub fn encode(input: &[u8], flags: u8) -> String {
  let mut buf = Default::default();
  encode_to(&mut buf, input, flags);
  buf
}

pub fn decode(input: &str) -> Option<Vec<u8>> {
  let mut buf = Default::default();
  decode_to(&mut buf, input).then_some(buf)
}

pub fn encode_to(buf: &mut String, input: &[u8], flags: u8) {
  let [url_safe, padding] = [flags & URL_SAFE != 0, flags & NO_PADDING == 0];
  let encode = move |sextet| sextet::encode(0x3f & sextet as u8, url_safe);

  // SAFETY: Encoded characters and padding bytes are valid ASCII / UTF-8.
  let buf = unsafe { buf.as_mut_vec() };
  let len_start = buf.len();
  let len_padded = (input.len() + 2) / 3 * 4;
  buf.reserve_exact(len_padded);

  let mut queue = 0u32;
  let mut count = 0u8;

  for &byte in input {
    queue = queue << 8 | byte as u32;
    count += 8;

    while let Some(rest) = count.checked_sub(6) {
      buf.push(encode(queue >> rest));
      queue &= !(!0 << rest);
      count = rest;
    }
  }

  if let shift @ ..6 = 6 - count {
    buf.push(encode(queue << shift));
  }

  if padding {
    buf.resize(len_start + len_padded, b'=');
  }
}

pub fn decode_to(buf: &mut Vec<u8>, input: &str) -> bool {
  let input = input.trim_end_matches('=');
  buf.reserve_exact(input.len() * 3 / 4);

  let mut queue = 0u32;
  let mut count = 0u8;

  for &byte in input.as_bytes() {
    let sextet @ ..64 = sextet::decode(byte) else { return false };
    queue = queue << 6 | sextet as u32;
    count += 6;

    if let Some(rest) = count.checked_sub(8) {
      buf.push((queue >> rest) as u8);
      queue &= (1 << rest) - 1;
      count = rest;
    }
  }

  count != 6
}

mod sextet {
  const ENCODE: [[u8; 2]; 0x100] = {
    let mut arr = [[0; 2]; 0x100];
    let mut i = 0;
    while i < 64 {
      let b = match i {
        b @ 0..=25 => b + b'A',
        b @ 26..=51 => b + b'a' - 26,
        b @ 52..=61 => b + b'0' - 52,
        _ => 0,
      };
      arr[i as usize] = [b, b];
      i += 1;
    }
    arr[62] = [b'+', b'-'];
    arr[63] = [b'/', b'_'];
    arr
  };

  const DECODE: [u8; 0x100] = {
    let mut arr = [!0; 0x100];
    let mut i = 0;
    while i < 64 {
      let [std, url] = ENCODE[i];
      arr[std as usize] = i as u8;
      arr[url as usize] = i as u8;
      i += 1;
    }
    arr
  };

  pub const fn encode(sextet: u8, url_safe: bool) -> u8 {
    ENCODE[sextet as usize][url_safe as usize]
  }

  pub const fn decode(byte: u8) -> u8 {
    DECODE[byte as usize]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rfc4648_test_vectors() {
    assert_eq!(encode(b"", 0), "");
    assert_eq!(encode(b"f", 0), "Zg==");
    assert_eq!(encode(b"fo", 0), "Zm8=");
    assert_eq!(encode(b"foo", 0), "Zm9v");
    assert_eq!(encode(b"foob", 0), "Zm9vYg==");
    assert_eq!(encode(b"fooba", 0), "Zm9vYmE=");
    assert_eq!(encode(b"foobar", 0), "Zm9vYmFy");

    assert_eq!(decode(""), Some(vec![]));
    assert_eq!(decode("Zg=="), Some(b"f".to_vec()));
    assert_eq!(decode("Zm8="), Some(b"fo".to_vec()));
    assert_eq!(decode("Zm9v"), Some(b"foo".to_vec()));
    assert_eq!(decode("Zm9vYg=="), Some(b"foob".to_vec()));
    assert_eq!(decode("Zm9vYmE="), Some(b"fooba".to_vec()));
    assert_eq!(decode("Zm9vYmFy"), Some(b"foobar".to_vec()));
  }

  #[test]
  fn unpadded_decode() {
    assert_eq!(decode("Zg"), Some(b"f".to_vec()));
    assert_eq!(decode("Zm8"), Some(b"fo".to_vec()));
    assert_eq!(decode("Zm9vYg"), Some(b"foob".to_vec()));
    assert_eq!(decode("Zm9vYmE"), Some(b"fooba".to_vec()));
  }

  #[test]
  fn url_safe() {
    let data = [251, 255, 254];
    assert_eq!(encode(&data, 0), "+//+");
    assert_eq!(encode(&data, URL_SAFE), "-__-");
    assert_eq!(encode(&data, URL_SAFE | NO_PADDING), "-__-");

    assert_eq!(decode("+//+"), Some(data.to_vec()));
    assert_eq!(decode("-__-"), Some(data.to_vec()));
  }

  #[test]
  fn roundtrip() {
    let input: [_; 0x100] = std::array::from_fn(|i| i as u8);

    for len in 0..0x100 {
      let input = &input[..len];

      let encoded = encode(input, 0);
      let decoded = decode(&encoded).expect("failed to decode valid base64");
      assert_eq!(input, decoded);

      let encoded = encode(input, URL_SAFE | NO_PADDING);
      let decoded = decode(&encoded).expect("failed to decode url safe base64");
      assert_eq!(input, decoded);
    }
  }

  #[test]
  fn invalid_input() {
    assert_eq!(decode("Z"), None);
    assert_eq!(decode("!@#$"), None);
    assert_eq!(decode("Zm9v\0"), None);
  }

  #[test]
  fn buffer_methods() {
    let mut buf = String::from("prefix: ");
    encode_to(&mut buf, b"hello", 0);
    assert_eq!(buf, "prefix: aGVsbG8=");

    let mut buf = b"prefix: ".to_vec();
    assert!(decode_to(&mut buf, "aGVsbG8="));
    assert_eq!(buf, b"prefix: hello");

    let mut buf = Vec::new();
    assert!(!decode_to(&mut buf, "invalid!!!"));
  }

  #[test]
  #[ignore]
  fn throughput() {
    // cargo test --release -p td-types throughput -- --ignored --nocapture
    assert!(!cfg!(debug_assertions), "no --release flag");

    use std::hint::black_box;
    use std::time::Instant;

    let payload = vec![0x42; 1024 * 1024]; // 1 MiB
    let encoded = encode(&payload, 0);
    let iterations = 100;
    let total_bytes = (payload.len() * iterations) as f64;

    let mut buf = String::new();
    let start = Instant::now();
    for _ in 0..iterations {
      buf.clear();
      encode_to(&mut buf, black_box(&payload), 0);
      black_box(&buf);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let encode_mib_s = (total_bytes / (1024.0 * 1024.0)) / elapsed;

    let mut buf = Vec::new();
    let start = Instant::now();
    for _ in 0..iterations {
      buf.clear();
      decode_to(&mut buf, black_box(&encoded));
      black_box(&buf);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let decode_mib_s = (total_bytes / (1024.0 * 1024.0)) / elapsed;

    println!("encoding: {encode_mib_s:.2} MiB/s");
    println!("decoding: {decode_mib_s:.2} MiB/s");
  }
}
