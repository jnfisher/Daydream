# A Daydream test server on Vultr

How to put a Daydream on the internet for testing: the web editor from any browser, and the
MCP endpoint for hosted agents (Claude.ai's and ChatGPT's connectors, the Claude and OpenAI
APIs). It serves the **stock 1.11p world** only. Never put a sysop's own realm on it: their
data stays on their machine.

What runs on the box:

- `daydream serve` as the `daydream` user, on `127.0.0.1:7340` only, with an access token
  (`/etc/daydream/env`, mode 600). Without a token Daydream refuses anything that comes
  through a proxy, so the token is not optional here.
- Caddy in front, on 80 and 443, getting a Let's Encrypt certificate for the host name and
  passing requests to Daydream. Caddy keeps no access log, since a connector's URL
  (`/mcp/<token>`) carries the token.
- ufw allowing SSH, 80 and 443; SSH key-only.
- The realm in `/var/lib/daydream/realm`, converted from the stock distribution (downloaded
  from MBBSEmu once and checked). Edits stay in it across restarts and updates.

[`tools/deploy/setup.sh`](../tools/deploy/setup.sh) does all of it, builds Daydream from
source on the box (no Rust needed at home), and is safe to run again to update.
[`tools/deploy/cloud-init.yaml`](../tools/deploy/cloud-init.yaml) runs it on the first boot.

## What you need

- A Vultr account and an SSH key added to it (Account, SSH Keys).
- A host name, one of:
  - **None**: leave `DAYDREAM_DOMAIN` blank and the server uses its address as
    `45-76-1-2.sslip.io`, which resolves to `45.76.1.2` with no DNS set up. Let's Encrypt
    issues certificates for these. Good for a throwaway box; the name changes with the box.
  - **Your own**: a name such as `daydream.example.com`, with a DNS A record pointing at the
    instance. Set it in `DAYDREAM_DOMAIN`. Create the record as soon as the instance has an
    address; the script keeps retrying for two minutes and Caddy keeps trying after that.

## 1. A firewall group (Vultr console)

Network, Firewall, Add Firewall Group, named `daydream`. Inbound IPv4 rules:

| Protocol | Port | Source |
|---|---|---|
| SSH | 22 | your address (`My IP`), or anywhere |
| TCP | 80 | anywhere (Let's Encrypt checks the name over port 80, and Caddy redirects it to 443) |
| TCP | 443 | anywhere (Claude.ai's and ChatGPT's servers call it from their own addresses) |

Add the same three for IPv6 if the instance gets an IPv6 address. Use a new group rather
than silvermere's: that one opens 2323, which Daydream does not use.

## 2. The instance (Vultr console)

Deploy, Cloud Compute (shared CPU), Chicago (or wherever is nearest).

- **Image**: Ubuntu 24.04 LTS or newer (Caddy comes from Ubuntu's own packages).
- **Plan**: 2 GB of memory or more. The first build compiles Daydream there and takes about
  10 minutes on 1 vCPU. A 1 GB plan works too: the script adds 2 GB of swap for the build,
  which makes it slower.
- **SSH Keys**: yours.
- **Firewall Group**: `daydream`.
- **Additional Features**: Cloud-Init User-Data. Paste
  [`cloud-init.yaml`](../tools/deploy/cloud-init.yaml) and edit its settings:
  - `DAYDREAM_DOMAIN`: blank for sslip.io, or your name.
  - `DAYDREAM_REF`: the branch to build. It must have the access token
    (`daydream new-token`); the script stops if it does not.
  - `DAYDREAM_SETUP_REF`: the branch this script comes from. Both are `main` once the
    access-token and deploy pull requests are merged.

The user data holds no secret. The token is made on the server and never leaves it unless
you copy it.

## 3. Watch it come up (SSH)

```sh
ssh root@HOST
tail -f /var/log/daydream-setup.log
```

It ends with `daydream setup exit 0` and a summary of the URLs. The checks before it should
say `daydream answers on 127.0.0.1:7340`, `without the token: HTTP 401` and
`https://NAME/ answers, with a certificate`.

Without cloud-init (or on any Ubuntu/Debian box), run the script by hand instead:

```sh
curl -fsSL https://raw.githubusercontent.com/jnfisher/daydream/main/tools/deploy/setup.sh -o setup.sh
sudo DAYDREAM_DOMAIN=daydream.example.com bash setup.sh
```

## 4. Use it

Read the token on the server; it is a password for the realm, so copy it straight into the
browser or connector, not into a chat or a ticket:

```sh
sudo grep DAYDREAM_TOKEN /etc/daydream/env
```

- **Browser**: open `https://NAME/` and sign in with the token once.
- **Claude Code** (from home):
  `claude mcp add --transport http daydream https://NAME/mcp --header "Authorization: Bearer $DAYDREAM_TOKEN"`
- **Claude.ai or ChatGPT custom connectors** that take only a URL:
  `https://NAME/mcp/<token>` with no authentication. The URL is the secret.

## Updating, resetting, rotating

All on the server, as root; each reruns the same script, which keeps its settings in
`/etc/daydream/deploy.env`:

```sh
bash /var/lib/daydream/src/tools/deploy/setup.sh                        # pull and rebuild DAYDREAM_REF
DAYDREAM_REF=some-branch bash /var/lib/daydream/src/tools/deploy/setup.sh   # try another branch
DAYDREAM_RESET_REALM=1 bash /var/lib/daydream/src/tools/deploy/setup.sh     # back to the stock world
DAYDREAM_NEW_TOKEN=1 bash /var/lib/daydream/src/tools/deploy/setup.sh       # new token
```

A reset moves the old realm aside to `realm.old-<time>` rather than deleting it. A new token
signs out every browser and breaks every connector until it is given the new one.

Logs: `journalctl -u daydream -u caddy`. Status: `systemctl status daydream caddy`.

## Taking it down

Destroy the instance in the Vultr console (and the DNS record, if you made one). Nothing
else holds state. Vultr bills a stopped instance as a running one, so destroy rather than
stop a box you are done with.
