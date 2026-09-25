# Project Instructions

notes_plugin_sicompass was split out of the
[sicompass](https://github.com/friendlyflow/sicompass) workspace, and its git
history before that point is the history of `lib/lib_notes` there. Work on it is
usually driven from a sicompass checkout next to this one (`../sicompass`), whose
`/commit-and-push`, `/release`, `/sync` and `/update-cargo` take this repo's name
as their first argument and then follow the skills in this repo's
`.claude/skills/`.

It is a sicompass **WASM plugin**: a `cdylib` built for `wasm32-wasip2` with
`sicompass-pdk`, installed by the sicompass Store from this repo's GitHub
releases. The plugin platform is described in
`../sicompass/docs/plugin-platform.md` and `../sicompass/docs/wasm-plugins.md`.

- `plugin.json` is the manifest. Its `name` (`notes`) is also the install
  folder, the settings section and the storage folder, and its `version` must
  equal the release tag. Permissions: `storage` (the notes, at `/storage` in the
  sandbox, which the host maps to `<data dir>/notes`, the folder the old
  built-in used, so existing notes open unchanged) and `allowedHosts
  ["store.sicompass.org"]` (the backup server). `service.tier` is
  `friendlyflow/cloud`, which is what lets `license::token` hand this plugin
  the user's Sicompass Cloud redeem token.
- `locales/<lang>.ftl`, every id prefixed `notes-`, in all four languages.
  `src/localize.rs` asks the host inside the sandbox and reads `en-US.ftl`
  natively, so the unit tests see the English text.
- `src/lib.rs` is the provider (`NotesProvider`, `impl Plugin`), `tree.rs` the
  hashed tree, `store.rs` the on-disk format, `escape.rs` the escaping every
  row goes through.
- `src/cloud.rs` is the optional cloud backup, off until the user ticks
  "enable cloud backup". It uses the `sicompass-payments` guest library.

## Cloud backup: three things that are easy to get wrong

- **The paywall is on the service, never on the data.** Whatever
  `license::standing` says, the notes are listed and saved to disk. Only the
  upload is gated (active or grace).
- **The backup row is rendered, never stored.** It carries `<id>cloud</id>`,
  and `reconcile` skips it. The app hands back whatever it displayed, so
  without that the row becomes a note. It never links anywhere: buying and
  redeeming are in the Store, under tiers.
- **Nothing slow runs in the UI instance.** `save` only marks the debounce.
  `poll` starts a `backup` task once the notes are quiet, and `restore` is a
  task too. A task runs in a fresh instance (`run_task`), so everything it needs
  comes from `/storage`, `license::token` and its `input`. Restore never runs
  over notes that exist, checked both in the UI and in the task.

## Environment (Nix)

The toolchain comes from the flake dev shell in [flake.nix](flake.nix): Rust
from rust-overlay with the `wasm32-wasip2` target (nixpkgs' rustc has no `std`
for it), `wasm-tools` and `jq`. Nothing is installed system-wide.

- **Check once per session**, then stick with the answer: `command -v cargo`.
  - Non-empty: the shell is inside `nix develop`, so run `cargo ...` directly.
  - Empty: prefix every toolchain command with `nix develop -c`.
- `nix develop -c <cmd>` prints a `warning: Git tree ... is dirty` line on
  stderr first. That warning is noise, not a failure.
- Evaluate the flake through `git+file://$PWD`, never a plain path (a plain path
  copies `target/` into the store and hangs), and always under `timeout`.
- The version lives in `plugin.json` and in `[package] version` in `Cargo.toml`.
  Bump both together.

## Generated files that are committed

- `THIRD-PARTY-LICENSES.html`: `cargo about generate about.hbs -o
  THIRD-PARTY-LICENSES.html` (cargo-about 0.9.2, the version the `licenses.yml`
  workflow pins). Regenerate and commit it with any dependency change. The
  workflow fails if it drifts.

## Code Style

Follow standard Rust idioms. Use `#[allow(...)]` sparingly and only when
justified. In `README.md`, do not use em dashes or semicolons. Use commas
instead, or split into separate sentences.

## Testing

- After implementing changes, always run the tests before finishing:
  `cargo test` (natively), and `./scripts/release-plugin.sh --dry-run`, which
  also builds the component and audits its imports.
- When adding new code, write or update tests.
- If tests fail, fix the code. Never leave a task with failing tests.

## Test Integrity

- Never remove or weaken test assertions to make a failing test pass. Fix the
  code instead.
- If a test itself is genuinely wrong and needs changing, **ask the user
  first** before modifying it.

## Releasing

A release is a `vX.Y.Z` tag on `main`, equal to `plugin.json`'s version. See
`.claude/skills/release/SKILL.md`. Before tagging, run
`nix develop -c ./scripts/release-plugin.sh --dry-run` (needs the
`sicompass-plugin` tool: `cargo install --git
https://github.com/friendlyflow/sicompass-plugin-sdk sicompass-plugin`). The
release workflow signs with the `PLUGIN_SIGNING_KEY` secret and checks it
against the `PLUGIN_PUBLIC_KEY` variable, the key the sicompass store list
names. The secret key file is `~/.config/sicompass/plugin-keys/notes.key`
on the maintainer's machine. Never print, copy or commit it.

The SDK and the pdk come from crates.io, and `sicompass-payments` by git at the
SDK's release tag (the source is all in `../sicompass-plugin-sdk`). The
commented-out `[patch]` in `Cargo.toml` is for working on them together, and
stays commented on main.
