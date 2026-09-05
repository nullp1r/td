# td

Telegram clients and bots in Rust, powered by [TDLib](https://core.telegram.org/tdlib).

`td` gives you typed requests, async calls, and a stream of updates without
hand-written JSON. TDLib handles the Telegram connection; you write the application.
This is a native client library, not an HTTP Bot API wrapper.

## What you get

- **The generated TDLib API.** Requests know their response types.
- **Messages you can await.** Wait for a send's final outcome, including individual album results.
- **File transfers you can follow.** Upload/download measurements and cooperative cancellation.
- **Room for your application.** Multiple clients, concurrent requests, and ordered updates—without a prescribed bot framework or retry policy.

## A quick look

With an authorized client:

```rust
use td_client::{client::Client, error::Result};
use td_types::{enums::User, fns};

async fn who_am_i(client: &Client) -> Result {
  let User::user(user) = client.sender().send(&fns::getMe {}).await?;
  println!("Hello from {}!", user.first_name);
  Ok(())
}
```

One client owns the session. Clone its sender for concurrent work, receive updates
through the owner, and call `shutdown().await` when you're done.

## How it fits together

`td-client` is the application-facing layer, `td-types` provides generated Rust
types, and `td-sys` connects to the native library. At build time, `td-parser` and
`td-codegen` turn TDLib's [TL schema](https://core.telegram.org/mtproto/TL) into Rust.

The crates are unpublished and the API is still being refined. Development uses local path
dependencies; `./td/fetch` downloads the native library and matching schema.

Inspired by Lonami's [grammers](https://codeberg.org/Lonami/grammers) and
[Telethon](https://codeberg.org/Lonami/Telethon).
