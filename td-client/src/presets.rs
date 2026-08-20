// https://github.com/thedemons/opentele/blob/main/docs/documentation/authorization/api.md
// https://github.com/thedemons/opentele/blob/main/src/api.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
  pub api_id: i32,
  pub api_hash: &'static str,
  pub device_model: &'static str,
  pub system_version: &'static str,
  pub app_version: &'static str,
  pub system_lang_code: &'static str,
}

pub const DESKTOP: Preset = Preset {
  api_id: 2040,
  api_hash: "b18441a1ff607e10a989891a5462e627",
  device_model: "Desktop",
  system_version: "Windows 10",
  app_version: "3.4.3 x64",
  system_lang_code: "en-US",
};

pub const ANDROID: Preset = Preset {
  api_id: 6,
  api_hash: "eb06d4abfb49dc3eeb1aeb98ae0f581e",
  device_model: "Samsung SM-G998B",
  system_version: "SDK 31",
  app_version: "8.4.1 (2522)",
  system_lang_code: "en-US",
};

pub const ANDROID_X: Preset = Preset {
  api_id: 21724,
  api_hash: "3e0cb5efcd52300aec5994fdfc5bdc16",
  device_model: "Samsung SM-G998B",
  system_version: "SDK 31",
  app_version: "8.4.1 (2522)",
  system_lang_code: "en-US",
};

pub const IOS: Preset = Preset {
  api_id: 10840,
  api_hash: "33c45224029d59cb3ad0c16134215aeb",
  device_model: "iPhone 13 Pro Max",
  system_version: "14.8.1",
  app_version: "8.4",
  system_lang_code: "en-US",
};

pub const MACOS: Preset = Preset {
  api_id: 2834,
  api_hash: "68875f756c9b437a8b916ca3de215815",
  device_model: "MacBook Pro",
  system_version: "macOS 12.0.1",
  app_version: "8.4",
  system_lang_code: "en-US",
};

pub const WEB_Z: Preset = Preset {
  api_id: 2496,
  api_hash: "8da85b0d5bfe62527e5b244c209159c3",
  device_model: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.110 Safari/537.36",
  system_version: "Windows",
  app_version: "1.28.3 Z",
  system_lang_code: "en-US",
};

pub const WEB_K: Preset = Preset {
  api_id: 2496,
  api_hash: "8da85b0d5bfe62527e5b244c209159c3",
  device_model: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.110 Safari/537.36",
  system_version: "Win32",
  app_version: "1.0.1 K",
  system_lang_code: "en-US",
};

pub const WEBOGRAM: Preset = Preset {
  api_id: 2496,
  api_hash: "8da85b0d5bfe62527e5b244c209159c3",
  device_model: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/96.0.4664.110 Safari/537.36",
  system_version: "Win32",
  app_version: "0.7.0",
  system_lang_code: "en-US",
};
