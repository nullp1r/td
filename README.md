# td

**Build full-featured Telegram clients in Rust with a fully typed interface to the entire TDLib API.**

`td` generates a typed Rust API from [TDLib]'s [API schema][td-api], written in Telegram's [Type Language (TL)][tl], and connects it to a small asynchronous client runtime. TDLib handles the protocol, synchronization, local storage, and media; `td` handles the Rust-facing boundary: request types, response correlation, ordered updates, message-send completion, and explicit shutdown.

This is a native TDLib integration, not an HTTP Bot API wrapper. It can power user clients, bots, and automation that need TDLib's full client capabilities.

Lonami's [grammers] and [Telethon] have both been a huge inspiration for this project.

## What you get

- Requests statically paired with their response types, generated from TDLib's [`td_api.tl`][td-api] schema.
- Concurrent requests and multiple independent clients routed through TDLib's single process-wide receiver.
- Ordered application updates with no library-defined dropping, retry, or logging policy.
- Retained message-send and file-transfer operations with observable progress, explicit awaiting, and awaited cancellation.
- One non-cloneable lifecycle owner, cloneable request-only senders, and fallible graceful shutdown.

## Architecture

![Build-time type generation feeding the asynchronous TDLib client runtime](assets/architecture.svg)

Most applications use only [`td-client`](td-client) and [`td-types`](td-types). The left side of the diagram runs at build time; the right side is the live request and update path.

## Get started

The bundled fetch path supports x86-64 and ARM64 on Linux with glibc and on macOS. It requires [Rust 1.98][rust], plus `curl`, `jq`, and `tar`; Linux also needs `readelf`.

Fetch the native library and its matching schema before building:

```bash
./td/fetch
```

The [`td/fetch`](td/fetch) script downloads the latest matching [`prebuilt-tdlib`][prebuilt-tdlib] package and [upstream schema][td-api]. Both artifacts stay local and are ignored by Git.

To run the echo bot, add [Telegram application credentials][telegram-apps] and a [bot token][botfather] to its ignored [configuration](td-client/examples/bot/config.example.json):

```bash
cp td-client/examples/bot/config.example.json td-client/examples/bot/config.json
cargo run -p td-client --example echo
```

The example authenticates a bot, logs incoming updates, and replies to text messages only after each reply reaches a terminal send result. Its complete source is [`td-client/examples/echo.rs`](td-client/examples/echo.rs).

## Core usage

Generated request structs carry their return type, so no response cast or hand-written JSON is needed:

```rust
use td_client::{Client, Result};
use td_types::enums::User;
use td_types::{fns, types};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result {
  let params = td_client::params(123456789, "api hash", ".td");
  let client = Client::bot(params, "bot token").await?;

  // keep application errors separate from lifecycle cleanup
  let result = identify(&client).await;
  let shutdown = client.shutdown().await;
  result?;
  shutdown
}

async fn identify(client: &Client) -> Result {
  let sender = client.sender();
  let User::user(types::user { id, first_name, .. }) = sender.send(&fns::getMe {}).await?;
  println!("signed in as {first_name} ({id})");
  Ok(())
}
```

The runtime deliberately has a small public vocabulary:

| Operation | Meaning |
| --- | --- |
| `Client::new` | Apply TDLib parameters; the caller drives authorization. |
| `Client::bot` | Apply parameters and complete the bot-token flow. |
| `Client::sender` | Create a cloneable, non-owning request capability. |
| `Sender::send` | Return a generated function's direct correlated response. |
| `Sender::send_message` / `send_messages` | Start retained normal-message send operations. |
| `Sender::track_message` | Attach another observer to a known pending send. |
| `Sender::upload` / `download` | Start retained preliminary-upload or exact-range download operations. |
| `Sender::track_file` | Observe coalesced transfer progress for a known file ID. |
| `Client::recv` | Receive the next ordinary update in TDLib order. |
| `Client::recv_auth` | Receive the next authorization state without losing ordinary updates. |
| `Client::shutdown` | Consume the owner and complete TDLib's close protocol. |
| `td_client::execute` | Run a TDLib function through the synchronous `td_execute` path. |

### Requests and updates

Clone `Sender` into detached tasks when work must continue independently of the update loop. A sender stores only a weak reference: it cannot receive updates, initiate shutdown, or keep a client alive. Requests that race with shutdown are either sent before `close` or rejected with `Error::Disconnected`.

One mutable `Client` drains the ordered update stream. `recv_auth` temporarily buffers non-authorization updates, and `recv` later returns them in their original order. Authorization transitions themselves are not exposed through the ordinary update stream.

The update queue is unbounded by design. TDLib's synchronous receiver cannot await capacity, and choosing which updates to delay, spill, or discard is application policy. Keep the receive loop moving and dispatch slow work separately.

### Message completion and cancellation

TDLib's direct response for a normal-message send can contain a temporary local message. `Sender::send_message` and `send_messages` return retained operations after that response has been atomically bound to terminal updates. Call `MessageSend::wait` for authoritative success, failure, or deletion. These entry points are only for actual non-preview normal sends; use `Sender::send` for previews, getters, and edits. Misrouted pending-looking responses may never settle.

```rust
let content = types::inputMessageText {
  text: types::formattedText { text: "hello".into(), ..Default::default() },
  ..Default::default()
};
let request = fns::sendMessage {
  chat_id,
  input_message_content: content.into(),
  ..Default::default()
};
let mut send = sender.send_message(&request).await?;
let message = send.wait().await?;
```

`MessageSend::cancel` consumes the operation, requests deletion only while its temporary ID is still pending, and awaits the terminal outcome. It returns `None` when deletion wins or `Some(final_message)` when authoritative success wins. It never explicitly deletes a successfully observed final message. This is not server-atomic: TDLib may itself delete a concurrently accepted message after removing its pending record.

Cancellation policy stays outside the crate. Race borrowed observation against any application signal, then drive the consuming cancellation future:

```rust
let mut send = sender.send_message(&request).await?;
tokio::select! {
  result = send.wait() => result.map(Some),
  () = cancelled() => send.cancel().await,
}
```

Dropping `MessageSend` performs no native work. Its pending entry remains tracked until TDLib emits a terminal update, allowing a caller that retained `send.key()` to attach again with `track_message`.

### File observation and transfers

File operations follow the same ownership model. Passive `FileWatch` values retain only copyable progress and coalesce observations; every original `updateFile` still remains in the application update queue. `track_file` is future-only and performs no implicit `getFile` request:

```rust
let mut watch = sender.track_file(file_id)?;
let progress = watch.wait(|progress| progress.download == td_client::TransferState::Completed).await?;
```

`Sender::download` forces TDLib's `downloadFile.synchronous` flag. This does not block the calling thread; it retains TDLib's asynchronous request promise until the requested full file or exact byte range is locally available. `Download::wait` consequently returns the authoritative file or failure:

```rust
let request = fns::downloadFile { file_id, priority: 16, offset: 0, limit: 0, ..Default::default() };
let mut download = sender.download(request)?;
let file = download.wait().await?;
```

`Sender::upload` awaits the direct `preliminaryUploadFile` response because that is where TDLib assigns the file ID. `Upload::wait` then reports the first non-active progress state, but does not call it success or failure: TDLib supplies no authoritative standalone preliminary-upload result. Completion belongs to the message or other operation that consumes the uploaded file.

The consuming `cancel` methods await the native cancellation ceremony. `Download::cancel` additionally awaits the original download response and returns `Some(file)` if completion won the race.

Dropping an operation abandons only local observation. It does not cancel work already submitted to TDLib, and explicit cancellation progresses only while its consuming future is polled.

### Synchronous execution

`td_client::execute(&request)` exposes TDLib's stateless synchronous path with the same generated request/response typing and error mapping as asynchronous sends. Only functions documented by TDLib as synchronously executable are meaningful there:

```rust
let mime_type = td_client::execute(&fns::getFileMimeType { file_name: "photo.jpg".into() })?;
```

The runtime serializes `td_execute` with `td_receive` and finishes parsing while holding that process-wide lock because either native call may invalidate TDLib's shared response buffer. An execute call can therefore wait for the configured native receive timeout.

### Authorization and shutdown

`Client::bot` is the narrow convenience path. For user accounts or custom authorization, construct with `Client::new`, read states with `recv_auth`, and send the corresponding generated authentication functions.

Always call `Client::shutdown`. It sends the generated `close` request, waits for `authorizationStateClosed`, revokes request senders, unregisters the client, and waits for the process-wide receiver to reach a safe handoff point. `Drop` intentionally performs no native work and does not claim a graceful shutdown.

## Generated API

[`td-types`](td-types) exposes four useful namespaces:

| Namespace | Contents |
| --- | --- |
| `fns` | Request structs such as `getMe` and `sendMessage`. |
| `types` | Concrete TDLib object structs. |
| `enums` | Tagged unions such as `Update`, `MessageContent`, and `User`. |
| `traits::Function` | The request-to-response type association used by `Sender::send`. |

Generated names intentionally preserve TDLib spelling, including lowercase constructor structs and variants. Objects and enums implement Serde serialization and deserialization; requests serialize and declare their deserializable return type. The wire adapters preserve TDLib's JSON representation for 64-bit integers, bytes, nullable fields, and tagged objects.

Detailed workspace API documentation is generated and published by [`td/docs`](td/docs).

## Workspace

| Crate | Responsibility |
| --- | --- |
| [`td-client`](td-client) | Typed requests, ordered updates, retained message and file operations, authorization, synchronous execution, and lifecycle. |
| [`td-types`](td-types) | Generated requests, responses, objects, updates, [Serde] implementations, and upstream documentation. |
| [`td-sys`](td-sys) | Raw unsafe bindings to TDLib's JSON C API plus native linking configuration. |
| [`td-parser`](td-parser) | A purpose-built, mostly borrowed parser for the [TDLib Type Language schema][td-api]. |
| [`td-codegen`](td-codegen) | Deterministic Rust generation and recursive-layout analysis. |

`td-parser`, `td-codegen`, and `td-types` form the build-time pipeline. The schema remains the source of generated code; generated output is never patched by hand.

## Build and test

After running [`td/fetch`](td/fetch), the normal repository checks need no Telegram credentials:

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

The ignored [live test](td-client/tests/live.rs) exercises a real bot and dedicated chat. Copy its [configuration](td-client/tests/live/config.example.json), provide `api_id`, `api_hash`, `bot_token`, and `chat_id`, and ensure [FFmpeg] is on `PATH` for generated media fixtures:

```bash
cp td-client/tests/live/config.example.json td-client/tests/live/config.json
cargo test -p td-client --test live -- --ignored --nocapture --test-threads=1
```

It covers text send/edit/delete, common media uploads, forced-document behavior, terminal update preservation, and cancellation. Authorization is retained in `td-client/tests/live/session`; delete that ignored directory when changing bot accounts.

To emit a standalone generated API file at `td/td_api.rs` for inspection, run the [code-generation integration test](td-codegen/tests/codegen.rs):

```bash
cargo test -p td-codegen --test codegen upstream
```

## Runtime design

TDLib multiplexes all modern JSON clients through one process-wide `td_receive` call, which must not run concurrently. [`td-client`](td-client) therefore owns one process-lifetime receiver thread. It routes each object by required `@client_id`, correlates direct responses by `@extra`, and sends uncorrelated updates to one queue per client. The thread parks when no clients remain.

Each live native client has exactly one non-cloneable `Client`. Its cloneable `Sender` values and the global router hold weak references, so neither can extend the native lifecycle. `shutdown(self)` uses Rust ownership to make the shutdown right unique instead of exposing a public lifecycle state machine.

Terminal message-send tracking is receiver-owned rather than update-loop-owned. The receiver binds a direct response's temporary `(chat_id, message_id)` before waking its request future, then observes matching success, failure, or deletion updates without removing them from the application stream. Send completion therefore progresses even while the application is not polling `Client::recv`.

The library preserves transport order and reports serialization, TDLib, message, transfer, and lifecycle failures. Retry, deadlines, flood-wait handling, logging, task supervision, and backpressure stay in the application. `set_receive_timeout` only tunes the next process-wide native receive wait and is not a request timeout.

Updating TDLib with [`td/fetch`](td/fetch) also updates the [schema][td-api] used to generate Rust. In addition to running the full suite, re-audit TDLib's direct-response-before-terminal-update ordering before accepting an upgrade, because terminal `sendMessage` correlation depends on that implementation behavior rather than a schema guarantee.

[botfather]: https://t.me/BotFather
[ffmpeg]: https://ffmpeg.org/
[grammers]: https://codeberg.org/Lonami/grammers
[prebuilt-tdlib]: https://www.npmjs.com/package/prebuilt-tdlib
[rust]: https://www.rust-lang.org/tools/install
[serde]: https://serde.rs/
[td-api]: https://github.com/tdlib/td/blob/master/td/generate/scheme/td_api.tl
[tdlib]: https://core.telegram.org/tdlib
[telegram-apps]: https://my.telegram.org/apps
[telethon]: https://codeberg.org/Lonami/Telethon
[tl]: https://core.telegram.org/mtproto/TL
