use std::ffi::{CStr, CString};
use std::sync::{Mutex, Once};

use td_types::traits::Function;
use td_types::{enums, fns, types};

static LOCK: Mutex<()> = Mutex::new(());
static SILENCE: Once = Once::new();

fn silence_logs() {
  // SAFETY: The call passes no pointers or borrowed storage.
  SILENCE.call_once(|| unsafe { td_sys::td_set_log_verbosity_level(0) });
}

fn create_client() -> i32 {
  silence_logs();

  // SAFETY: The call takes no arguments and returns an opaque ID by value.
  unsafe { td_sys::td_create_client_id() }
}

fn send_and_receive<F: Function>(client_id: i32, req: &F) -> F::Return {
  let _guard = LOCK.lock().expect("lock poisoned");
  let req_json = serde_json::to_string(req).expect("serialize request");
  let c_req = CString::new(req_json).expect("valid CString");

  // SAFETY: `client_id` came from TDLib; `c_req` is live and NUL-terminated.
  unsafe { td_sys::td_send(client_id, c_req.as_ptr()) };

  loop {
    // SAFETY: `LOCK` serializes all `td_receive` and `td_execute` calls.
    let res_ptr = unsafe { td_sys::td_receive(1.0) };
    assert!(!res_ptr.is_null(), "timeout waiting for response");

    // SAFETY: `res_ptr` is non-null, NUL-terminated, and valid while `LOCK` is held.
    let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");

    if let Ok(ret) = serde_json::from_str::<F::Return>(res_str) {
      return ret;
    }
  }
}

fn execute_sync<F: Function>(req: &F) -> F::Return {
  silence_logs();
  let _guard = LOCK.lock().expect("lock poisoned");

  let req_json = serde_json::to_string(req).expect("serialize request");
  let c_req = CString::new(req_json).expect("valid CString");

  // SAFETY: `c_req` is live and NUL-terminated; this request supports sync execution.
  let res_ptr = unsafe { td_sys::td_execute(c_req.as_ptr()) };
  assert!(!res_ptr.is_null(), "td_execute response should not be null");

  // SAFETY: `res_ptr` is non-null, NUL-terminated, and valid while `LOCK` is held.
  let res_str = unsafe { CStr::from_ptr(res_ptr) }.to_str().expect("valid utf-8 response");
  serde_json::from_str::<F::Return>(res_str).expect("deserialize return type")
}

#[test]
fn call_empty() {
  let client_id = create_client();
  let res = send_and_receive(client_id, &fns::testCallEmpty {});
  assert_eq!(res, enums::Ok::ok);
}

#[test]
fn call_string() {
  let client_id = create_client();
  let input = "Hello Telegram FFI! 🦀🚀".to_string();
  let res = send_and_receive(client_id, &fns::testCallString { x: input.clone() });
  assert_eq!(res, types::testString { value: input }.into());
}

#[test]
fn call_bytes() {
  let client_id = create_client();
  let input = b"\x00\x01\x02\xFF\xFEbinary-test".to_vec();
  let res = send_and_receive(client_id, &fns::testCallBytes { x: input.clone() });
  assert_eq!(res, types::testBytes { value: input }.into());
}

#[test]
fn call_vector_int() {
  let client_id = create_client();
  let input = vec![-100, 0, 42, 1337, i32::MAX];
  let res = send_and_receive(client_id, &fns::testCallVectorInt { x: input.clone() });
  assert_eq!(res, types::testVectorInt { value: input }.into());
}

#[test]
fn call_vector_int_object() {
  let client_id = create_client();
  let input = vec![types::testInt { value: 7 }, types::testInt { value: 42 }];
  let res = send_and_receive(client_id, &fns::testCallVectorIntObject { x: input.clone() });
  assert_eq!(res, types::testVectorIntObject { value: input }.into());
}

#[test]
fn call_vector_string() {
  let client_id = create_client();
  let input = vec!["apple".into(), "banana".into(), "cherry".into()];
  let res = send_and_receive(client_id, &fns::testCallVectorString { x: input.clone() });
  assert_eq!(res, types::testVectorString { value: input }.into());
}

#[test]
fn call_vector_string_object() {
  let client_id = create_client();
  let input = vec![types::testString { value: "first".into() }, types::testString { value: "second".into() }];
  let res = send_and_receive(client_id, &fns::testCallVectorStringObject { x: input.clone() });
  assert_eq!(res, types::testVectorStringObject { value: input }.into());
}

#[test]
fn square_int() {
  let client_id = create_client();
  let res = send_and_receive(client_id, &fns::testSquareInt { x: 10 });
  assert_eq!(res, types::testInt { value: 100 }.into());
}

#[test]
fn return_error_synchronously() {
  let err = types::error { code: 418, message: "I'm a teapot".into() };
  let res = execute_sync(&fns::testReturnError { error: err.clone() });
  assert_eq!(res, err.into());
}

#[test]
fn use_update() {
  let client_id = create_client();
  let res = send_and_receive(client_id, &fns::testUseUpdate {});
  // Verify that a valid Update variant is returned
  let is_update = matches!(res, enums::Update::updateOption(_));
  assert!(is_update, "expected Update option variant, got: {res:?}");
}
