# Project Instructions

notes-plugin-sicompass was split out of the
[sicompass](https://github.com/friendlyflow/sicompass) workspace, and its git
history before that point is the history of `lib/lib_notes` there. Work on it is
usually driven from a sicompass checkout next to this one (`../sicompass`), whose
`/commit-and-push`, `/release`, `/sync` and `/update-cargo` take this repo's name
as their first argument and then follow the skills in this repo's
`.claude/skills/`.

It is a sicompass **plugin process**: a program (`src/main.rs`) built with the
SDK's `plugin` feature, which sicompass starts and talks to over its stdin and
stdout. It runs with the user's rights. The Store installs it from this repo's
GitHub releases, one build per platform. The plugin platform is described in
`../sicompass/docs/plugin-platform.md`.

- `plugin.json` is the manifest. Its `name` (`notes`) is also the install
  folder, the settings section and the storage folder, and its `version` must
  equal the release tag. Permissions, which declare what the plugin does and
  are shown to the user before install: `storage` (the notes, in
  `sicompass_sdk::plugin::storage_dir()`, which is `<data dir>/notes`, the
  folder the old built-in used, so existing notes open unchanged) and
  `allowedHosts ["store.sicompass.org"]` (the backup server). `service.tier` is
  `friendlyflow/cloud`, which is what lets `license::token` hand this plugin
  the user's Sicompass Cloud redeem token.
- `locales/<lang>.ftl`, every id prefixed `notes-`, in all four languages.
  `src/localize.rs` asks the app (`host::translate`), and in the unit tests,
  which run outside sicompass, reads `en-US.ftl`, so they see the English text.
- `src/lib.rs` is the provider (`NotesProvider`, `impl Plugin`), and
  `src/main.rs` makes it the program. `tree.rs` is the hashed tree, `store.rs` the on-disk format, `escape.rs` the escaping every
  row goes through.
- `src/cloud.rs` is the optional cloud backup, off until the user ticks
  "enable cloud backup". It uses the `sicompass-payments` library, with a
  host of its own (`PluginHost`): the app through the plugin kit, threads for
  the tasks, and `ureq` (rustls) for the HTTP.

## Cloud backup: three things that are easy to get wrong

- **The paywall is on the service, never on the data.** Whatever
  `license::standing` says, the notes are listed and saved to disk. Only the
  upload is gated (active or grace).
- **The backup row is rendered, never stored.** It carries `<id>cloud</id>`,
  and `reconcile` skips it. The app hands back whatever it displayed, so
  without that the row becomes a note. It never links anywhere: buying and
  redeeming are in the Store, under tiers.
- **Nothing slow runs on the calls from the app.** The app waits for
  every call to answer. `save` only marks the debounce. `poll` starts a `backup` task once
  the notes are quiet, and `restore` is a task too. A task runs on a thread of
  its own (`PluginHost::spawn`), with only the notes folder on disk, the token
  and its `input`. `poll` hands its result to `NotesProvider::task_done`.
  Restore never runs over notes that exist, checked both in the UI and in the
  task.

## Environment (Nix)

The toolchain comes from the flake dev shell in [flake.nix](flake.nix): Rust
from rust-overlay with this computer's plugin target (static musl on Linux,
which nixpkgs' rustc has no std for) and `jq`. Nothing is installed
system-wide.

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
  `cargo test`, and `./scripts/release-plugin.sh --dry-run`, which also builds
  this computer's release and verifies it the way the Store will.
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

The SDK comes from crates.io, and `sicompass-payments` by git at the SDK's
release tag (the source is all in `../sicompass-plugin-sdk`). The
commented-out `[patch]` in `Cargo.toml` is for working on them together, and
stays commented on main.

A release has one archive per platform. The release workflow builds them on
five runners (Linux x86_64 and arm64 as static musl, macOS arm64 and x86_64,
Windows x86_64), then packs, signs and verifies them in one job.
