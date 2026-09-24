# notes_plugin_sicompass

*Your own notes, in Sicompass.*

This plugin is part of [Sicompass](https://github.com/friendlyflow/sicompass), a
keyboard-first, accessibility-first way to use your entire computer.

Notes is a tree of lists you write in. A note can hold a list of its own, which
can hold more lists, as deep as you like. You add, rename, move, cut and paste
notes with the same keys as everywhere else in Sicompass, and every change can
be undone.

Every list opens with a list meta row. It shows a SHA-256 hash of that list and
everything below it, so a glance at the top of your notes tells you whether
anything anywhere has changed. Inside a note's own list meta you can set it to
private or public. New notes are private.

Your notes are plain files in your Sicompass data folder, and nothing else can
read them.

## Cloud backup

Cloud backup is off until you turn it on, in Settings, under notes. With it on,
a row at the top of your notes says where your subscription stands, and a copy
of your notes goes to the Sicompass Cloud server a few seconds after you stop
typing.

Cloud backup is part of Sicompass Cloud, which you buy and redeem in the Store,
under tiers. Without it your notes work exactly the same, and are only not
copied to the server. After a subscription ends, backup keeps running for 14
more days.

To get your notes back on a new computer, turn cloud backup on and run restore
cloud backup from the command palette. It only restores into empty notes, and
never over notes you already have.

## Install

In Sicompass, open store, then programs, and press Enter on install next to
notes. The Store checks the release's signature before installing it, and keeps
it up to date.

## Building from source

```bash
nix develop          # the toolchain, with the wasm32-wasip2 target
cargo test           # the notes and the backup logic, natively
cargo build --release --target wasm32-wasip2
cp target/wasm32-wasip2/release/notes_plugin.wasm plugin.wasm
```

`./scripts/release-plugin.sh --dry-run` does the build, checks the component
against `plugin.json`, and signs and verifies it with a throwaway key, the way
a release is made.

## Related repositories

- [sicompass](https://github.com/friendlyflow/sicompass), the application
- [sicompass-plugin-sdk](https://github.com/friendlyflow/sicompass-plugin-sdk),
  the SDK and the WASM plugin kit
- [payments_plugin_sicompass](https://github.com/friendlyflow/payments_plugin_sicompass),
  the cloud backup library

## Community

Join the conversation on
[Discord](https://discord.com/channels/1464152138753249313/1464152139231137894).

## License

#### Open source license

If you are creating an open source application under a license compatible with
the GNU GPL license v3, you may use this project under the terms of the GPLv3.
See [LICENSE](LICENSE).

## Contributing

Contributions are welcome. Whether it is code, documentation, or feedback, your
input helps make computing more accessible for everyone.
