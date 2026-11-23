# A Daydream test server over Tailscale Funnel

How to put a Daydream running on your own Linux machine or Mac on the internet for testing: the web
editor from any browser, and the MCP endpoint for hosted agents (Claude.ai's and ChatGPT's
connectors, the Claude and OpenAI APIs). Those agents call from their own servers, which are
not on your tailnet, so plain Tailscale is not enough; Funnel gives the machine a public
`https://<machine>.<tailnet>.ts.net` address, with a certificate, and no ports opened on
your router. Funnel is on every Tailscale plan, the free one included.

It serves the **stock 1.11p world** only. Never put a sysop's own realm on it.

What [`tools/deploy/funnel.sh`](../tools/deploy/funnel.sh) sets up, all as your own user:

- Daydream built from source (rustup is installed for you if there is no Rust), the binary
  in `~/.local/bin/daydream`, the source in `~/.local/share/daydream/src`.
- The realm in `~/.local/share/daydream/realm`, converted from the stock distribution
  (downloaded from MBBSEmu once and checked). Edits stay in it across restarts and updates.
- An access token in `~/.config/daydream/env` (mode 600). Funnel adds forwarding headers, and
  Daydream refuses forwarded requests without a token, so the token is not optional.
- `daydream serve` on `127.0.0.1:7340` as a user service: on Linux a systemd **user** unit,
  with lingering so it runs without you logged in and comes back after a reboot; on macOS a
  launchd agent (`~/Library/LaunchAgents/io.github.jnfisher.daydream.plist`), which runs
  while you are logged in, logging to `~/Library/Logs/daydream/`.
- `tailscale funnel` from port 443 to it, kept across reboots by Tailscale.

Nothing else on the machine is exposed: Funnel forwards only that one port, and Daydream
asks for the token on everything but its sign-in page and health check.

## What you need

- Linux with systemd, `git`, `curl` and a C compiler (Debian/Ubuntu:
  `sudo apt install git curl build-essential`); or macOS with Apple's command line tools
  (`xcode-select --install`).
- Tailscale installed and logged in (`tailscale up`, or signed in to the app), with MagicDNS and HTTPS certificates on
  for the tailnet (admin console, DNS). The first `tailscale funnel` prints a link if Funnel
  is not yet allowed for the machine: open it, allow it, and the command carries on.
- About 10 minutes for the first build.

### On a Mac

- Use Tailscale's **standalone** app (tailscale.com/download/mac) or Homebrew's `tailscale`,
  not the Mac App Store build: Funnel from the command line may not work in the App Store
  one. The script finds the CLI on the `PATH` or inside `/Applications/Tailscale.app`.
- It needs no `sudo`.
- launchd runs Daydream only while you are logged in. For a Mac left to serve, keep it awake
  (System Settings, Energy: prevent automatic sleeping when the display is off) and turn on
  automatic login if it should come back by itself after a restart.

## Run it

```sh
curl -fsSL https://raw.githubusercontent.com/jnfisher/daydream/main/tools/deploy/funnel.sh -o funnel.sh
bash funnel.sh
```

On Linux it asks for `sudo` twice at most: `loginctl enable-linger` (once), and `tailscale funnel`
unless you are Tailscale's operator (`sudo tailscale set --operator=$USER` once avoids it).

It ends with the URLs. Check from outside the tailnet, for example on a phone with wifi off:
`https://<machine>.<tailnet>.ts.net/` shows the sign-in page.

If something else already uses `tailscale serve` on 443 on this machine, the script stops
rather than replace it: run it with `DAYDREAM_FUNNEL_PORT=8443` (or `10000`), Funnel's other
ports, and the URL gains `:8443`.

## Use it

The token is a password for the realm. Copy it straight into the browser or the connector,
not into a chat or a ticket:

```sh
grep DAYDREAM_TOKEN ~/.config/daydream/env
```

- **Browser**: open `https://<machine>.<tailnet>.ts.net/` and sign in with the token once.
- **Claude Code**:
  `claude mcp add --transport http daydream https://<machine>.<tailnet>.ts.net/mcp --header "Authorization: Bearer $DAYDREAM_TOKEN"`
- **Claude.ai or ChatGPT custom connectors** that take only a URL:
  `https://<machine>.<tailnet>.ts.net/mcp/<token>` with no authentication. The URL is the
  secret.

## Updating

Setup installs the script as `~/.local/bin/daydream-funnel` and remembers its settings
(branch, Funnel port) in `~/.config/daydream/deploy.env`, so updating is one command:

```sh
daydream-funnel update
```

It fetches the branch and, when it moved, lists the new commits, rebuilds, and restarts
Daydream (a few seconds of downtime). When nothing changed it says so and stops. The realm,
the token, the port and the funnel are left alone, so browsers and connectors keep working.
A build that fails leaves the running Daydream as it was.

To follow `main` without thinking about it:

```sh
daydream-funnel auto-update on     # checks every 15 minutes (DAYDREAM_UPDATE_EVERY=1h for hourly)
daydream-funnel auto-update off
journalctl --user -u daydream-update    # what the last runs did (macOS: ~/Library/Logs/daydream/update.log)
```

`~/.local/bin` is on the `PATH` of most distributions once it exists (log in again after the
first setup); otherwise run `~/.local/bin/daydream-funnel`.

## Other branches, resetting, rotating, stopping

```sh
DAYDREAM_REF=some-branch bash funnel.sh   # build a branch (remembered for later updates);
                                          # DAYDREAM_REF=main bash funnel.sh goes back
DAYDREAM_RESET_REALM=1 bash funnel.sh     # back to the stock world (the old realm is moved aside)
DAYDREAM_NEW_TOKEN=1 bash funnel.sh       # new token: every browser and connector needs it again
daydream-funnel off                       # stop the funnel, the service and auto-update
```

Here `funnel.sh` is `daydream-funnel` or any copy of the script. A commit given as
`DAYDREAM_REF` must be the full hash.

Logs: `journalctl --user -u daydream` (macOS: `~/Library/Logs/daydream/serve.log` and
`update.log`). Status: `systemctl --user status daydream` (macOS:
`launchctl print gui/$UID/io.github.jnfisher.daydream`) and `tailscale funnel status`.
