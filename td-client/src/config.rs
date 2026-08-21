use serde::Serialize;
use td_types::fns;

use crate::presets::Preset;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
  pub td: fns::setTdlibParameters,
}

impl Default for Config {
  fn default() -> Self {
    Self { td: defaults() }
  }
}

impl From<fns::setTdlibParameters> for Config {
  fn from(td: fns::setTdlibParameters) -> Self {
    Self { td }
  }
}

impl From<Preset> for Config {
  fn from(preset: Preset) -> Self {
    Self { td: preset.into() }
  }
}

impl From<Preset> for fns::setTdlibParameters {
  fn from(p: Preset) -> Self {
    Self {
      api_id: p.api_id,
      api_hash: p.api_hash.into(),
      device_model: p.device_model.into(),
      system_version: p.system_version.into(),
      application_version: p.app_version.into(),
      system_language_code: p.system_lang_code.into(),
      ..defaults()
    }
  }
}

pub fn defaults() -> fns::setTdlibParameters {
  fns::setTdlibParameters {
    database_directory: ".td/db".into(),
    files_directory: ".td/files".into(),
    use_file_database: true,
    use_chat_info_database: true,
    use_message_database: true,
    system_language_code: "en".into(),
    device_model: "Server".into(),
    application_version: env!("CARGO_PKG_VERSION").into(),
    ..Default::default()
  }
}
