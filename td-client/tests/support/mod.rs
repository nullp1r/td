//! Native client fixtures and deliberate serialization faults.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, process};

use serde::Serialize;
use serde::ser::Error as _;
use td_types::traits::Function;
use td_types::{enums, fns};

const API_ID: i32 = 2040;
const API_HASH: &str = "b18441a1ff607e10a989891a5462e627";

/// Fails before a request can reach `TDLib`.
pub struct BadRequest;

impl Serialize for BadRequest {
  fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
    Err(S::Error::custom("intentional failure"))
  }
}

impl Function for BadRequest {
  type Return = enums::Ok;
}

/// Sends valid wire data but deliberately declares the wrong response type.
pub struct WrongReturn;

impl Serialize for WrongReturn {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    fns::testSquareInt { x: 2 }.serialize(serializer)
  }
}

impl Function for WrongReturn {
  type Return = enums::Ok;
}

pub fn test_parameters(root: &Path, name: &str) -> fns::setTdlibParameters {
  let directory = root.join(name);
  fns::setTdlibParameters {
    api_id: API_ID,
    api_hash: API_HASH.into(),
    database_directory: directory.join("db").to_string_lossy().into_owned(),
    files_directory: directory.join("files").to_string_lossy().into_owned(),
    system_language_code: "en".into(),
    device_model: "td-client test".into(),
    application_version: "td-client test".into(),
    ..Default::default()
  }
}

pub fn test_root() -> PathBuf {
  let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock predates Unix").as_nanos();
  env::temp_dir().join(format!("td-client-{}-{nonce}", process::id()))
}
