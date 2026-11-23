# A Daydream test server over Tailscale Funnel

How to put a Daydream running on your own Linux machine on the internet for testing: the web
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
- `daydream serve` on `127.0.0.1:7340` as a systemd **user** service, with lingering so it
  runs without you logged in and comes back after a reboot.
- `tailscale funnel` from port 443 to it, kept across reboots by Tailscale.

Nothing else on the machine is exposed: Funnel forwards only that one port, and Daydream
asks for the token on everything but its sign-in page and health check.

## What you need

- Linux with systemd, `git`, `curl` and a C compiler (Debian/Ubuntu:
  `sudo apt install git curl build-essential`).
- Tailscale installed and logged in (`tailscale up`), with MagicDNS and HTTPS certificates on
  for the tailnet (admin console, DNS). The first `tailscale funnel` prints a link if Funnel
  is not yet allowed for the machine: open it, allow it, and the command carries on.
- About 10 minutes for the first build.

## Run it

```sh
curl -fsSL https://raw.githubusercontent.com/jnfisher/daydream/main/tools/deploy/funnel.sh -o funnel.sh
bash funnel.sh
```

It asks for `sudo` twice at most: `loginctl enable-linger` (once), and `tailscale funnel`
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

## Updating, resetting, rotating, stopping

Each reruns the same script; settings can also live in `~/.config/daydream/deploy.env`.

```sh
S=~/.local/share/daydream/src/tools/deploy/funnel.sh
bash $S                                # pull and rebuild main
DAYDREAM_REF=some-branch bash $S       # try another branch
DAYDREAM_RESET_REALM=1 bash $S         # back to the stock world (the old realm is moved aside)
DAYDREAM_NEW_TOKEN=1 bash $S           # new token: every browser and connector needs it again
bash $S off                            # stop the funnel and the service
```

Logs: `journalctl --user -u daydream`. Status: `systemctl --user status daydream` and
`tailscale funnel status`.
