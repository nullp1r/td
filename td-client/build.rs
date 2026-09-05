//! Links `td-client` to the native `TDLib` library.

use td_sys::build;

fn main() {
  build::link();
}
