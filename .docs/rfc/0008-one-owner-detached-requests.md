# RFC 0008: One owner, detached requests

- Status: Implemented as a checked prototype
- Scope: Fresh replacement of `td-client`
- Compatibility: Not a goal
- Prototype: [`td-client.v5`](../../td-client.v5)
- Decision: Keep one non-`Clone` lifecycle owner and add one non-owning request-only capability

## Result

The tokfu bot demonstrates the first requirement that the current API cannot meet cleanly: one task must consume the ordered update stream while independently spawned message handlers and long-lived background jobs issue Telegram requests.

The replacement API therefore has two capabilities, but still only one owner:

```rust
pub struct Client; // owns updates and native lifecycle; not Clone

pub struct Requests; // can only execute requests

impl Client {
  pub async fn new(parameters: setTdlibParameters) -> Result<Self>;
  pub async fn bot(parameters: setTdlibParameters, token: &str) -> Result<Self>;
  pub fn requests(&self) -> Requests;
  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return>;
  pub async fn recv(&mut self) -> Result<Option<Update>>;
  pub async fn auth(&mut self) -> Result<AuthorizationState>;
  pub async fn shutdown(self) -> Result<()>;
}

impl Requests {
  pub async fn execute<F: Function>(&self, request: &F) -> Result<F::Return>;
}
```

`Requests` is the one addition justified by a real caller. It cannot receive updates or initiate shutdown. `Client` remains the sole owner, and shutdown still consumes it.

The prototype is 382 lines of production Rust including `build.rs`, compared with the current implementation's 332. The 50-line increase buys three required properties together:

- detached request tasks;
- a request/close gate with a defined race outcome;
- local revocation when the owner is dropped, even while request capabilities survive.

The second rules/line-count pass removed 14 production lines from the first checked v5 prototype while also eliminating blocking destructor locks and per-response weak-registry pruning.

It adds no dependency, runtime task, public state enum, builder, session wrapper, or actor.

## Regression audit against the current client

The prototype was rechecked against every `td-client` change after the RFC 0007-era implementation, not only against RFC 0007's embedded code.

| Current fix | v5 result |
| --- | --- |
| Construction and bot-auth failures attempt shutdown while preserving the original error | Retained |
| `shutdown` unregisters and waits for the receiver transition even when close/event processing fails | Retained; an intervening event error is now remembered while the terminal state is still awaited |
| A previously observed `authorizationStateClosed` suppresses a second close request | Retained |
| Non-auth updates consumed during authentication are replayed in order | Retained |
| Request serialization, wrong return-type deserialization, and TDLib `error` responses remain distinct failures | Retained and covered by the integration test |
| Same-client concurrency, cross-client routing, simultaneous shutdown, shutdown/registration races, and worker restart are tested | Retained |
| Runtime responsibilities are separated into client, state, and router operations | Retained without restoring the older module/type hierarchy |
| `Client: Debug` avoids dumping internal state | Retained without exposing the native transport ID |
| Rust 1.97 and the workspace lint policy | Retained |
| Public receive-timeout tuning | Deliberately removed: it exposes worker synchronization and no demonstrated consumer owns that policy; the native receive wait remains a private implementation bound |

The audit also removes a current hot-path cost that v5 no longer needs. Current routing prunes the complete weak registry before every lookup. V5 lookup is one expected-O(1) `HashMap::get` plus `Weak::upgrade`; graceful shutdown removes its entry explicitly, while abandoned entries are pruned once after their weak state dies rather than scanned on every response.

## Evidence from tokfu

This RFC treats the linked application as the consumer specification, not as code to imitate internally.

| Application behavior | Evidence | Wrapper consequence |
| --- | --- | --- |
| Update handling, autoposting, and archiving run concurrently | [`App::run`](../ref/tokfu-bot/crates/app/src/app.rs) selects three long-lived futures | All three need concurrent access to one TDLib client |
| Each incoming message starts independent work | [`handle_message_update`](../ref/tokfu-bot/crates/app/src/app/messages.rs) clones `App` into `tokio::spawn` | A borrowed `&Client` cannot enter the required `'static` future |
| Message work performs requests after network and blocking work | [`handle_links`](../ref/tokfu-bot/crates/app/src/app/links.rs) downloads, uploads, edits, and replies | Serializing the update loop around a mutable client would stall unrelated updates for seconds or minutes |
| Background jobs send while the update loop remains active | [`archiving`](../ref/tokfu-bot/crates/app/src/app/archiving.rs) and [`autoposting`](../ref/tokfu-bot/crates/app/src/app/channel.rs) | A single sequential handler is not an acceptable migration |
| The application shares its Telegram capability through `Arc<Inner>` | [`App` and `Inner`](../ref/tokfu-bot/crates/app/src/app.rs) | The request capability must be `Send + Sync` and storable in the existing `Arc`; it need not be `Clone` or own updates or shutdown |
| Upload and send completion can arrive as later updates | [`upload`](../ref/tokfu-bot/crates/app/src/upload.rs) currently relies on higher-level grammers behavior | The base wrapper must preserve every update; final-message workflows belong in tokfu's Telegram adapter |

The current `td-client` deliberately disallowed detached request actors until a real consumer required them. That ceiling has now been reached.

## Consumer shape

The rewrite can keep tokfu's existing concurrency without making the native client cloneable:

```rust
struct Inner {
  tg: td_client::Requests,
  // configuration, HTTP client, rate limiter, caches, ...
}

let mut client = td_client::Client::bot(parameters, &token).await?;
let app = App::new(client.requests()).await?;

let result = app.run(&mut client).await;
app.cancel_and_join_children().await;
drop(app);
let shutdown = client.shutdown().await;

result?;
shutdown
```

`App::run` owns the application policy: signals, branch cancellation, child-task tracking, rate limiting, and logging. `td-client` only guarantees that a request racing shutdown is either sent before `close` or rejected as disconnected.

Tokfu currently detaches message tasks and does not retain their join handles. Its rewrite should track them so application work has an intentional shutdown policy. The wrapper does not wait for all holders of `Arc<Inner>`: a forgotten task would otherwise turn graceful shutdown into an ownership-count deadlock. Instead, shutdown revokes the shared request capability immediately.

## Ownership model

```text
Client (one owner, not Clone)
  ├─ ordered event receiver
  ├─ auth buffer
  └─ Arc<State>
       ├─ request gate + pending responses
       ├─ event sender
       └╌ Weak<State> ─ Requests in Arc<Inner> ─┬─ event handlers
                                                ├─ archiver
                                                └─ autoposter

Router ── Weak<State> only
```

Only `Client` stores a strong `Arc<State>`. `Requests` stores `Weak<State>` and upgrades it for the duration of one request:

- only `Client` owns the event receiver;
- only `Client::shutdown(self)` can claim graceful native shutdown;
- dropping `Client` closes its existing event receiver, which revokes new detached requests without running destructor code;
- `Requests` never closes TDLib, keeps idle state alive, or keeps a router entry alive;
- the router stores `Weak<State>`, so it cannot manufacture ownership; an already-running request may keep state alive only until that request finishes.

This distinction is why the new type is named `Requests`, not `ClientHandle`. It describes the granted operation instead of suggesting that a clone represents the client session.

## Request and shutdown ordering

Detached request execution creates one new race that RFC 0007 did not have:

```text
task A: execute(request)       owner: shutdown(close)
```

An atomic `closing` check would be insufficient. A task could pass the check, be suspended, and call `td_send` after `close`. The prototype instead folds one `open` bit into the existing pending-request mutex and reuses event-receiver closure to detect an abandoned owner:

```rust
struct RequestState {
  open: bool,
  pending: HashMap<u64, oneshot::Sender<Result<Vec<u8>>>>,
}
```

Ordinary execution performs:

1. allocate `@extra` and serialize the typed request;
2. lock `RequestState`;
3. reject the request if `open` is false or the sole `Client` event receiver has gone away;
4. install its pending response sender;
5. call `td_send` while still holding the short synchronous lock;
6. unlock and await the request-local oneshot.

Shutdown performs the same sequence, except its owner-only close operation changes `open` to false before installing and sending typed `fns::close {}`. The close path remains available if the event receiver itself failed, allowing TDLib to receive the native close even though its terminal update can no longer be observed.

The mutex therefore defines a total send order:

- a request that acquires the lock first is sent before `close`;
- a request that acquires it afterwards returns `Error::Disconnected` without touching TDLib.

No synchronous lock is held across `.await`. No actor, command channel, task-local registry, shutdown state machine, or request-state atomic is needed.

When the client reaches its terminal state, `disconnect` closes the gate and drops every remaining pending sender, so their receivers report `Disconnected`. When the owner is merely abandoned, new requests are rejected but requests already sent may still receive their routed responses; the last such request releases the state. A canceled request future is harmless: its pending entry is removed when the native response arrives, and sending to the dropped oneshot is ignored.

## Update ownership

The update side does not become cloneable. One `Client` still owns one unbounded, ordered queue.

`recv(&mut self)` filters authorization transitions and returns application updates in TDLib order. `auth(&mut self)` returns the next authorization state and buffers every intervening application update in a `VecDeque`. The bot constructor uses `auth`, so startup cannot lose updates that precede `authorizationStateReady`.

The queue remains unbounded. The native receiver thread cannot await Tokio capacity, and neither dropping nor reordering updates is a generic library policy. Tokfu's event loop drains this queue promptly and moves slow work into tasks, which is exactly the access pattern an unbounded handoff requires.

### Send and upload completion

TDLib functions do not all have the same completion semantics. In particular, `sendMessage` can return a local pending message and later emit `updateMessageSendSucceeded` or `updateMessageSendFailed`; file work emits `updateFile`.

The wrapper must not turn those domain updates into speculative generic futures. Tokfu can implement the behavior it actually needs in its Telegram adapter:

```text
spawned handler ── execute(sendMessage) ──> local message id
       │
       └── await application oneshot <── event dispatcher
                                           ├─ updateMessageSendSucceeded
                                           └─ updateMessageSendFailed
```

That correlation table is application state because its timeout, retry, persistence, and cancellation rules are tokfu policy. `td-client`'s responsibility is to deliver the updates without loss. A reusable typed helper can be promoted later only if multiple consumers demonstrate the same semantics.

## Router and receiver

The modern TDLib JSON interface multiplexes every live client through process-wide `td_receive`. The replacement retains:

- exactly one receiver thread;
- a `HashMap<i32, Weak<State>>` selected by required `@client_id`;
- one pending `HashMap<u64, ...>` per client selected by `@extra`;
- one event queue per client;
- a parked process-lifetime worker when the registry is empty.

Multiple simultaneous clients remain supported. Tokfu itself uses one, but multi-client routing is an established repository requirement and now costs only the registry already required by the chosen TDLib interface.

The worker lifecycle keeps RFC 0007's single `watch::Sender<()>` transition signal. Final shutdown subscribes before removing its entry and marks the current version as seen under the registry lock. The next change means either:

- the worker observed the empty registry between native receives and is about to park; or
- another client registered and now owns the reason for continued receiving.

In both cases the closing client is no longer reachable and the process-wide receiver has crossed a safe ownership boundary.

The public receive-timeout setter is removed. It exposed a worker polling detail for which neither tokfu nor the example application has a policy requirement. The private one-second native wait only bounds how soon the worker acknowledges an empty registry; it does not delay available updates.

## Protocol errors

TDLib guarantees `@client_id` and `@type`, so the borrowed envelope models them as required fields. A malformed envelope cannot be attributed to one client. Silently discarding it would violate the library's error-preservation contract, so the router publishes the JSON error to every live event queue.

To make that rare broadcast possible without converting errors to strings, the JSON payload is stored behind `Arc`; `Error` itself remains non-`Clone`. The success path gains no allocation or copy. Errors that have a valid client ID, including malformed update bodies and TDLib `error` objects, remain local to that client.

Responses for canceled request futures are the one intentional non-event: the router removes the pending entry and finds its oneshot receiver gone. There is no caller left to receive that response.

## Lifecycle

### Construction

`Client::new` creates and registers a state, then sends generated `setTdlibParameters`. If it fails, construction attempts the ordinary graceful shutdown and returns the original setup error.

### Bot authorization

`Client::bot` reacts only to the states required for bot login. Any unexpected state is returned as `Error::Auth`. Failure again attempts graceful shutdown before preserving the original authentication error.

Interactive authorization needs no public session or callback abstraction. A caller reads `auth()` and sends the corresponding generated TDLib functions.

### Explicit shutdown

`shutdown(self)`:

1. closes the request gate and sends generated `fns::close {}` through correlated request execution;
2. propagates the close response;
3. records the first intervening event error but keeps receiving until `authorizationStateClosed`;
4. fails any remaining pending requests;
5. removes the router entry and awaits the receiver's safe idle/ownership transition;
6. returns the first lifecycle failure.

If the terminal state was already observed, shutdown does not send a second close.

### Drop

`Client` needs no `Drop` implementation. Ordinary field destruction closes the event receiver and drops the owner's `Arc<State>` without calling TDLib, locking, sleeping, joining, or reporting successful shutdown. Detached requests then fail their existing sender-closure check.

An already-sent request may temporarily retain `State` and finish normally. When the last strong reference disappears, `State::drop` performs one atomic dirty notification; the receiver thread prunes the now-dead weak router entry on its next loop. No destructor takes a mutex.

Dropping `Client` without `shutdown` is still misuse. The library makes stale Rust capabilities safe; it cannot complete an asynchronous native close protocol from a destructor.

## Reconsidered alternatives

### Keep only borrowed `&Client`

Rejected by the demonstrated message-handler `tokio::spawn` calls. Replacing them with sequential handling would block update draining during downloads and uploads. A local `FuturesUnordered` could borrow the client, but would force the application to rebuild structured task scheduling solely around a wrapper limitation.

### Make `Client: Clone`

Rejected because every clone would appear to own receiving and shutdown. Consuming one clone could not prove that another would stop issuing requests, so the API would need a public lifecycle state machine or last-owner convention.

### Return `(ClientHandle, UpdateReceiver)`

Rejected because neither value would clearly own native shutdown. The design would recreate the paired-resource problem from RFCs 0001-0003.

### Put `Client` behind `Arc<tokio::sync::Mutex<_>>`

Rejected because `recv().await` would hold the mutex indefinitely and exclude every request. Splitting locks inside the public type recreates `Requests` less explicitly and less ergonomically.

### Add a request actor

Rejected because TDLib already supports concurrent `td_send`. A task plus command enum and response erasure would add scheduling, allocations, dynamic typing, and another failure boundary only to serialize an operation that needs a short mutex for shutdown ordering.

### Broadcast or subscribe to all updates

Rejected because it invents lag, duplication, filtering, and overflow policy. One ordered owner plus application-level routing matches tokfu's dispatcher.

### Wait for all request-capability holders during shutdown

Rejected because an `Arc<Inner>` stored in a detached task or a cycle could deadlock shutdown. Revocation is both smaller and operationally safer.

### Add file, message-delivery, retry, or flood-wait helpers

Rejected at this layer. The generated operations and updates are sufficient; tokfu already owns rate-limit and retry policy. Promote only a workflow whose invariant proves reusable during the actual rewrite.

### Restore `execute_sync`

Rejected. There is still no caller, and TDLib's global stateless response buffer requires process-wide synchronization with the active receiver to be safe.

## Deliberate public surface

Kept:

- `Client`, `Requests`, `Error`, and `Result`;
- typed `execute`, ordered `recv`, arbitrary `auth`, bot convenience, and consuming `shutdown`;
- generated `setTdlibParameters` plus a small `defaults()` value;
- TDLib's global log-verbosity setter.

Absent:

- native client IDs;
- a builder or configuration wrapper;
- a public receiver, stream adapter, auth session, shutdown guard, or lifecycle enum;
- tracing and retry policy;
- bounded update overflow behavior;
- a receive-timeout knob;
- synchronous execution;
- compatibility shims for RFC 0007.

## Repository-guideline audit

- Consumer need: `Requests` exists only for tokfu's demonstrated detached handlers and background jobs. It is not `Clone`, stores only `Weak<State>`, and cannot own lifecycle, updates, or shutdown.
- Ownership: one non-`Clone` `Client` stores the sole durable `Arc<State>`; borrowed `&Client` remains the ordinary concurrent-request path; `&mut Client` consumes ordered updates; shutdown consumes `Client`.
- Protocol: all operations are generated typed TDLib functions; `close` uses the correlated request path; required `@client_id` and `@type` fields are non-optional.
- Error and ordering preservation: the update queue is unbounded and ordered, auth buffering uses `VecDeque`, request and TDLib errors are propagated, malformed unrouteable envelopes are broadcast as JSON errors, and shutdown retains the first event failure while still seeking the terminal state.
- Synchronization: the sole receiver is one parked OS thread; the request/close gate and pending map share one short `std::sync::Mutex`; replies use `oneshot`; ordered events use `mpsc`; worker transitions use `watch`; no synchronous guard crosses `.await`.
- Policy: there is no tracing, retry, flood-wait, overflow, message-delivery, or public timeout policy in the library.
- Efficiency: routing and response correlation use expected-O(1) `HashMap`s, authentication buffering is contiguous, request serialization stays in bytes, and owner revocation reuses the existing event channel instead of allocating another signal.
- FFI: dynamic requests are explicitly NUL-terminated, every `unsafe` block has its local invariant, and destructors perform no native or blocking work.
- Dependencies and language: v5 adds no dependency, inherits workspace versions with only Tokio `sync`, targets Edition 2024/Rust 1.97, and keeps the strict workspace lints clean.

## Efficiency

The successful request path performs the operations inherent to the JSON API: one serialization buffer, one pending-map entry, one oneshot, one native send, one response copy, and one typed deserialization.

The request gate reuses the pending-map mutex. Holding it through the nonblocking `td_send` call removes a race without another request-state atomic or lock. Owner-drop detection reuses `UnboundedSender::is_closed`, so it adds no channel or allocation. Tokfu stores one `Requests` value in `Arc<Inner>`; cloning `App` only increments that existing application `Arc`.

Router lookup is expected O(1): one `HashMap::get` and one `Weak::upgrade`. A single dirty bit triggers weak-entry pruning only after state destruction, avoiding the current implementation's full-registry scan for every response.

Updates are deserialized once into generated owned types and moved through a contiguous channel queue. Authentication buffering allocates only for updates that actually arrive during authentication.

## Verification

The prototype contains four focused unit tests and one real TDLib integration test.

They cover:

- exact `@extra` request serialization;
- same-extra cross-client routing;
- TDLib error routing;
- ordinary update routing;
- malformed-envelope error broadcast;
- auth filtering and preservation of intervening update order;
- revocation of a detached request capability after owner drop;
- terminal-state observation before an intervening event error is returned;
- cleanup after the event receiver disconnects;
- failed initialization followed by successful clients;
- request serialization and return-type deserialization errors;
- detached `tokio::spawn` request execution;
- concurrent same-client and cross-client requests;
- an already-closed client;
- shutdown while the non-owning `Requests` capability remains alive;
- simultaneous shutdown of multiple clients;
- a request racing shutdown;
- registration racing final shutdown;
- worker reuse after shutdown;
- one deadline around the native lifecycle test.

The checked source of truth is:

- [`td-client.v5/src/lib.rs`](../../td-client.v5/src/lib.rs)
- [`td-client.v5/src/tests.rs`](../../td-client.v5/src/tests.rs)
- [`td-client.v5/tests/client.rs`](../../td-client.v5/tests/client.rs)

The prototype passes formatting, compilation, its real TDLib tests, and the workspace's strict Clippy configuration on Rust 1.97.1. The complete lifecycle integration test also passed in 10 consecutive fresh test processes.
