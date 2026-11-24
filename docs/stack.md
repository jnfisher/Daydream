# The in-game test stack (`daydream-stack`)

One command sets up everything needed to check a Daydream edit in the game, on a Linux or
macOS machine:

- **silvermere**, the game server, from its newest GitHub release, on `127.0.0.1:2323`, with
  the stock MajorMUD 1.11p world;
- a **test character**: the account `tester` (a sysop with the `DEBUG` permission, so the
  developer commands such as `SYSOP EXPRESS_TRAIN` and `SYSOP SUMMON` work) and its level 70
  Dwarf Warrior (a Dwarf sees in the dark), every stat at the race's maximum, wearing the best stock armour a Warrior
  can use, made with `silvermere character create` (silvermere 1.0.2 or later);
- **Daydream**, from its newest release (built from source while it has none), on
  `http://127.0.0.1:7350/`, finding that silvermere game;
- **SyncTERM**, a BBS terminal with a real CP437 font and full ANSI, with a dialing-directory
  entry `daydream-stack` that logs in as the test account.

```sh
curl -fsSLO https://raw.githubusercontent.com/jnfisher/daydream/main/tools/stack/stack.sh
bash stack.sh init          # installs itself as ~/.local/bin/daydream-stack
```

Everything lives in `~/.local/share/daydream-stack/` (the release binaries, silvermere's
folder and database, Daydream's workspace). The settings, including the test password, are in
`~/.config/daydream-stack/stack.env` (mode 600). The services are systemd user units on Linux
(`daydream-stack-silvermere`, `daydream-stack-daydream`) and launchd agents on macOS
(`io.github.jnfisher.daydream-stack.*`, logs in `~/Library/Logs/daydream-stack/`). It never
touches `~/silvermere` or a `daydream-funnel` install, and uses its own ports.

## Checking an edit in the game

silvermere holds a single-writer lock on its database, and Daydream takes the same lock, so
Daydream writes only while silvermere is stopped:

1. Play in SyncTERM and find what to change.
2. `daydream-stack edit`: stops silvermere, which saves every character first.
3. In Daydream, open the silvermere game from the game picker, make the edit, write it back.
4. `daydream-stack play`: starts silvermere and opens SyncTERM again, logged in.

## Commands

| Command | What it does |
|---|---|
| `init` | Everything above, then prints the addresses and the login. Safe to run again. |
| `up [silvermere\|daydream]`, `down [...]` | Start or stop the services (both, or one). |
| `connect` | Open SyncTERM on the `daydream-stack` entry. |
| `edit`, `play` | Stop silvermere for an edit; start it again and reconnect. |
| `seed` | Make the test character again from the settings (stops silvermere for it). |
| `update [silvermere\|daydream\|syncterm] [--version V \| --rollback]` | The newest (or given) release, checked against its `SHA256SUMS`; silvermere's database is backed up first. SyncTERM through Homebrew or apt. |
| `reset` | Back up, then the stock world again and a fresh character. |
| `status` | Versions, running state, addresses and the login. |
| `logs silvermere\|daydream` | Follow a log. |
| `remove` | Stop and remove the services; the data stays. |

## Settings

Put them in `stack.env` or the environment, then run `init` (or `seed` for the character):

| Setting | Default |
|---|---|
| `STACK_TELNET_PORT`, `STACK_ADMIN_PORT`, `STACK_DAYDREAM_PORT` | 2323, 8081, 7350 |
| `STACK_USER`, `STACK_PASSWORD` | `tester`, a random password |
| `STACK_NAME`, `STACK_RACE`, `STACK_CLASS`, `STACK_LEVEL`, `STACK_GOLD` | Tester, Dwarf (night vision), Warrior, 70, 500 (coins weigh, so more than the pack can take is refused) |
| `STACK_ITEMS` | stock items separated by `;` (stormhammer, aegis shield, mithril chainmail hauberk, starsteel greathelm, ...) |
| `STACK_CLIENT_CMD` | a command to run in place of SyncTERM |
| `STACK_SILVERMERE_BIN` | a silvermere binary to use in place of the release (a build not yet released) |
| `STACK_STOCK_ZIP` | a saved `WCCMMUD_1.11p_DOS_MBBSEmu_v2.zip`, for when its download is blocked (see below) |
| `DAYDREAM_STACK_HOME` | `~/.local/share/daydream-stack` |

## When the stock download is blocked

`init` and `reset` build the stock world from MajorMUD 1.11p's MBBSEmu package, which silvermere
downloads once. A network that inspects HTTPS (a company laptop, say) can stop that download with
an error such as `invalid peer certificate: UnknownIssuer`. Get
`WCCMMUD_1.11p_DOS_MBBSEmu_v2.zip` another way (a browser, another network), then any of these
works:

- leave it in `~/Downloads`, where `init` looks for it;
- run with `STACK_STOCK_ZIP=/path/to/WCCMMUD_1.11p_DOS_MBBSEmu_v2.zip`;
- or just run `init`: when the download fails it asks for the file's path (dragging the file into
  Terminal works), and tries again.

The file's checksum is checked before it is used, and the path is saved in `stack.env`.

## SyncTERM

`init` installs it with Homebrew (`brew install --cask syncterm`) on macOS or apt on Linux, and
otherwise points at [syncterm.bbsdev.net](https://syncterm.bbsdev.net/). The entry goes into
SyncTERM's dialing directory (`~/.syncterm/syncterm.lst` on Linux,
`~/Library/Preferences/SyncTERM/syncterm.lst` on macOS); other entries are kept. Should
SyncTERM stop at `Username:` instead of logging in, type the login `daydream-stack status`
prints.
