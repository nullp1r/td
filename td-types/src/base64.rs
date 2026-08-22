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
  let encode = move |sextet| ENCODE[0x3f & sextet as usize][url_safe as usize];

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
    let sextet @ ..64 = DECODE[byte as usize] else { return false };
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

const ENCODE: [[u8; 2]; 0x100] = {
  let mut arr = [[0; _]; _];
  let mut i = 0;
  while i < 62 {
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
  let mut arr = [!0; _];
  let mut i = 0;
  while i < 64 {
    let [std, url] = ENCODE[i];
    arr[std as usize] = i as u8;
    arr[url as usize] = i as u8;
    i += 1;
  }
  arr
};

#[cfg(test)]
mod tests {
  use std::array;

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

    assert_eq!(decode("+//+"), Some(data.to_vec()));
    assert_eq!(decode("-__-"), Some(data.to_vec()));
  }

  #[test]
  fn roundtrip() {
    let input: [_; 0x100] = array::from_fn(|i| i as u8);
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
  #[ignore = "benchmark"]
  #[expect(clippy::assertions_on_constants, reason = "release only")]
  fn throughput() {
    use std::hint::black_box;
    use std::time::Instant;

    // cargo test --release -p td-types throughput -- --ignored --nocapture
    assert!(!cfg!(debug_assertions), "must be run with `--release`");

    let iters = 100;
    let mib = 1024 * 1024;
    let payload = (0..mib).map(|i| i as u8).collect::<Vec<_>>();
    let encoded = encode(&payload, 0);

    let (mut buf_enc, mut buf_dec): (String, Vec<u8>) = Default::default();

    let t0 = Instant::now();
    for _ in 0..iters {
      buf_enc.clear();
      encode_to(&mut buf_enc, black_box(&payload), 0);
      black_box(&buf_enc);
    }
    let t1 = Instant::now();
    for _ in 0..iters {
      buf_dec.clear();
      decode_to(&mut buf_dec, black_box(&encoded));
      black_box(&buf_dec);
    }
    let t2 = Instant::now();

    let total_mib = (iters * payload.len()) as f64 / mib as f64;
    let encode_mib_s = total_mib / t1.duration_since(t0).as_secs_f64();
    let decode_mib_s = total_mib / t2.duration_since(t1).as_secs_f64();

    println!("encoding: {encode_mib_s:.2} MiB/s");
    println!("decoding: {decode_mib_s:.2} MiB/s");
  }
}
