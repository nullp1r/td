# td

Telegram clients and bots in Rust, powered by
[TDLib](https://core.telegram.org/tdlib).

`td` gives you typed requests, async calls, and a stream of updates without
hand-written JSON. TDLib handles the Telegram connection; you write the
application. This is a native client library, not an HTTP Bot API wrapper.

## What you get

- **The generated TDLib API.** Requests know their response types.
- **Messages you can await.** Wait for a send's final outcome, including
  individual album results.
- **File transfers you can follow.** Upload/download measurements and
  cooperative cancellation.
- **Room for your application.** Multiple clients, concurrent requests, and
  ordered updates—without a prescribed bot framework or retry policy.

## A quick look

With an authorized session:

```rust
use td_client::types::{enums::User, fns};
use td_client::{Client, Result};

async fn who_am_i(client: &Client) -> Result {
  let User::user(user) = client.send(&fns::getMe {}).await?;
  println!("Hello from {}!", user.first_name);
  Ok(())
}
```

One session owns the TDLib instance and database. Clone its client for
concurrent work, receive updates through the session, and call `close().await`
when you're done.

## Ergonomic helpers with tdx

`tdx` re-exports the client and generated API, so it can be your application's
single Telegram dependency. Its helpers return editable TDLib request values:

```rust
use tdx::prelude::*;

async fn greet(client: &Client, chat_id: i64) -> tdx::client::Result<()> {
  let request = send::message(chat_id, line(("Hello, ", bold("world"), "!")));
  let sent = client.track(&request, None, None).await?;

  client.send(&edit::text(&sent, "Welcome back.".text())).await?;
  Ok(())
}
```

Use tuples with `line`/`lines` for composed text and `.text()` for plain strings. Media payloads
can be passed directly to send helpers; optional fields use explicit conversions,
such as `video.caption = Some(text.into())`. Albums and forwards expose each
message's outcome through `track_all`. Edits use `send`.

The [showcase](tdx/examples/showcase.rs) demonstrates formatting, keyboards,
quotes, targeting, media and generated requests. It takes an existing client;
its `main` performs no I/O.

For a runnable bot, copy `tdx/examples/config.example.json` to
`tdx/examples/config.json`, fill in your credentials, and run
`cargo run -p tdx --example workflow`. The [workflow](tdx/examples/workflow.rs)
authorizes a bot, replies and edits, and closes the session on Ctrl-C.
It stores authorization in `.tdx-session`.

Session defaults and bot authorization live in `tdx::session`:

```rust
let params = tdx::session::parameters(api_id, api_hash, "session");
let session = tdx::session::bot(params, token).await?;
```

Native logging accepts a safe capturing Rust closure and can change verbosity
while sessions run:

```rust
tdx::client::set_log_callback(|level, message| {
  eprintln!("TDLib [{level}]: {}", message.to_string_lossy());
});
tdx::client::set_log_level(2)?;
// Later:
tdx::client::set_log_level(4)?;
```

Installing the callback leaves the native output stream unchanged. It receives
`&CStr`; the application chooses how to decode and format it. Both log and error
callbacks are boxed, serialized, and removable through `clear_log_callback()` and
`clear_error_callback()`. Use `set_error_callback()` for unsolicited client errors.
All these functions are grouped in `client::diagnostics` and re-exported at its root.
Callbacks must not call TDLib or reconfigure callbacks.

Build the API documentation with `cargo doc -p tdx --no-deps` and check the
showcase with `cargo check -p tdx --example showcase`.

## How it fits together

`td-client` owns sessions, native logging and request/update routing. [`tdx`](tdx/src/lib.rs) adds small
parameter defaults, bot authorization, constructors, text formatting and borrowed accessors over the generated
`td-types` values. `td-sys` connects to the native library. At build time, `td-parser`
and `td-codegen` turn TDLib's [TL schema](https://core.telegram.org/mtproto/TL)
into Rust.

The crates are unpublished and the API is still being refined. Development uses
local path dependencies; `./td/fetch` downloads the native library and matching
schema.

Inspired by Lonami's [grammers](https://codeberg.org/Lonami/grammers) and
[Telethon](https://codeberg.org/Lonami/Telethon).

## Rustwater

The workspace also contains [`game/`](game/README.md), **Rustwater**: the Telegram-native persistent sandbox MMO RPG that serves as a demanding real application of `tdx` and TDLib. Its durable product, game-design, UX, narrative/art, Telegram and architecture documentation starts at [`game/docs/README.md`](game/docs/README.md).

## Testing

`cargo test --workspace` runs local tests, including native client routing and
shutdown. The `tdx` API tests are grouped by formatting, request construction, and
inspection. Credentialed Telegram tests and throughput benchmarks stay ignored.

To run the live scenarios, copy `tdx/tests/live/config.example.json` to
`tdx/tests/live/config.json` and supply a dedicated test chat and bot credentials.
With FFmpeg installed, run:

```sh
cargo test -p tdx --test live -- --ignored --nocapture --test-threads=1
```

The runner keeps authorization in `tdx/tests/live/session`, generates temporary
media, and attempts message cleanup and graceful shutdown even after a scenario
returns an error. Each run exercises sends, edits, album progress, downloads, and
cancellation. It can leave remote messages behind if a send times out before its
final message ID is known.
