# notes-plugin-sicompass

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

Your notes are plain files in your Sicompass data folder, on your own
computer.

## Cloud sync

Cloud sync is off until you turn it on, in Settings, under notes. With it on,
a row at the top of your notes says where your subscription stands, and your
notes stay the same on every computer you turn it on for. A few seconds after
you stop typing, and once a minute otherwise, Sicompass sends your changes to
the Sicompass Cloud server and brings in the changes you made elsewhere.

Every note has a hash, shown in its list meta, that changes whenever anything
in it changes. That is how Sicompass and the server know which notes are out
of date, and the list meta also says whether a list changed since the last
sync. When the same line was changed on two computers, the latest change is
kept. A note deleted on one computer and edited on another is kept.

Cloud sync is part of Sicompass Cloud, which you buy and redeem in the Store,
under tiers. Without it your notes work exactly the same, and are only not
synced. After a subscription ends, sync keeps running for 14 more days.

On a new computer, turn cloud sync on, and your notes arrive. Sync with the
cloud now, in the command palette, does it at once.

## Install

In Sicompass, open store, then programs, and press Enter on install next to
notes. The Store checks the release's signature before installing it, and keeps
it up to date.

## Building from source

```bash
nix develop          # the toolchain
cargo test           # the notes and the sync logic
cargo build --release
cp target/release/notes-plugin plugin
```

To install a build of your own, copy `plugin.json`, the built `plugin` program
(`plugin.exe` on Windows) and `locales/` into a folder named `notes` in the
Sicompass plugins folder (`~/.config/sicompass/plugins/` on Linux,
`~/Library/Application Support/sicompass/plugins/` on macOS) and restart
Sicompass.

`./scripts/release-plugin.sh --dry-run` builds this computer's release, packs
it, and signs and verifies it with a throwaway key, the way a release is made.

## Related repositories

- [sicompass](https://github.com/friendlyflow/sicompass), the application
- [sicompass-plugin-sdk](https://github.com/friendlyflow/sicompass-plugin-sdk),
  the SDK, the plugin kit and the cloud sync library (`sicompass-sync`)

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
