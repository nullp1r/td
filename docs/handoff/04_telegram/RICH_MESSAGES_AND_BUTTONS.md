# Rich Messages and In-Message Buttons

> **Status:** current target; lost `tdx` implementation needs recovery/reimplementation

## Design role

Rich Messages are not cosmetic decoration. They give the bot an actual application UI hierarchy inside Telegram.

The intended distinction is:

- content blocks communicate state/context;
- Rich Message buttons hold the primary contextual action;
- ordinary inline reply markup holds persistent navigation/secondary actions.

## `tdx` work reported as implemented in the lost tree

The un-packaged UX pass reportedly added helpers conceptually like:

```rust
button_row([
  success_callback_button("🎣 Cast a line", payload),
  callback_button("🔎 Explore", payload),
])
```

with default/primary/success/danger styles.

It also added analogous styling helpers to conventional markup, conceptually:

```rust
markup::primary(...)
markup::success(...)
markup::danger(...)
```

Exact final names are not sacred. Preserve the principle from root `AGENTS.md`: add the smallest ergonomic layer required by real callers, return/use generated TDLib types, and do not create a parallel UI type system.

## Button style semantics

Suggested convention:

- **primary:** main non-destructive action;
- **success:** positive/commit action such as Cast/Reel/Claim when appropriate;
- **danger:** destructive/escape action such as Cut Line;
- **default/link:** neutral navigation/secondary actions.

Do not color-code arbitrarily; styles should create predictable affordances.

## Testing

`tdx` tests should verify the exact generated Rich Message block/button representation and callback payload retention.
