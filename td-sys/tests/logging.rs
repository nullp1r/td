//! Exercises the native `TDLib` logging callback.

use std::ffi::{CStr, CString, c_char};
use std::fmt;

use td_types::{enums, fns};

fn log(lvl: i32, msg: fmt::Arguments<'_>) {
  let levels = [(1, "FATAL"), (1, "ERROR"), (3, "WARN"), (2, "INFO"), (4, "DEBUG"), (5, "TRACE")];
  let idx = usize::try_from(lvl).unwrap_or(0);
  let (ansi, tag) = levels.get(idx).map_or((0, "?"), |&s| s);
  println!("\x1b[3{ansi}m{tag:5}\x1b[39m {msg}");
}

unsafe extern "C" fn log_callback(lvl: i32, msg: *const c_char) {
  // SAFETY: TDLib supplies a non-null, NUL-terminated string valid for this call.
  let msg = unsafe { CStr::from_ptr(msg) }.to_str().unwrap_or("?");
  let msg = msg.split_once('\t').map_or(msg, |(_, rhs)| rhs).trim_end();
  log(lvl, format_args!("{msg}"));
}

#[test]
fn log_callback_receives_messages() {
  // SAFETY: The path is static and NUL-terminated. The callback has C ABI and
  // static lifetime, and calls no TDLib function.
  unsafe {
    td_sys::td_set_log_file_path(cr"/dev/null".as_ptr());
    td_sys::td_set_log_verbosity_level(5);
    td_sys::td_set_log_message_callback(5, Some(log_callback));
  }

  // SAFETY: The call takes no arguments and returns an opaque ID by value.
  let client_id = unsafe { td_sys::td_create_client_id() };
  assert!(client_id > 0);

  let req = fns::getOption { name: "version".into() };
  let req_json = serde_json::to_vec(&req).expect("serialize getOption request");
  let c_req = CString::new(req_json).expect("valid CString");

  // SAFETY: `client_id` came from TDLib; `c_req` is live and NUL-terminated.
  unsafe { td_sys::td_send(client_id, c_req.as_ptr()) };

  loop {
    // SAFETY: This is the only `td_receive` loop in the test process.
    let res_ptr = unsafe { td_sys::td_receive(1.0) };
    if res_ptr.is_null() {
      continue;
    }

    // SAFETY: `res_ptr` is non-null, NUL-terminated, and used before the next receive.
    let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");

    if let Ok(enums::OptionValue::optionValueString(val)) = serde_json::from_str(res_str) {
      assert_eq!(val.value.split('.').count(), 3);
      break;
    }

    if let Ok(update) = serde_json::from_str::<enums::Update>(res_str) {
      log(4, format_args!("{update:?}"));
    }
  }

  // SAFETY: No borrowed storage crosses either call.
  unsafe {
    td_sys::td_set_log_message_callback(1, None);
    td_sys::td_set_log_verbosity_level(1);
  }
}
