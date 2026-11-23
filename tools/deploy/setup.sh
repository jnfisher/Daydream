#!/usr/bin/env bash
# Set up (or update) a Daydream test server on a fresh Ubuntu or Debian VPS, such as a Vultr
# instance: build daydream from source, convert the stock MajorMUD 1.11p world into a realm,
# run `daydream serve` on 127.0.0.1 with an access token, and put Caddy in front of it for
# HTTPS. See docs/deploy-vultr.md.
#
# Run as root. Safe to run again: it updates the source and the binary, keeps the realm and
# the token, and rewrites the unit and the Caddyfile.
#
# Settings, from the environment or /etc/daydream/deploy.env:
#   DAYDREAM_DOMAIN       the host name to serve (its DNS A record points here). Blank: this
#                         machine's address as <a-b-c-d>.sslip.io, which needs no DNS.
#   DAYDREAM_REF          the branch, tag or commit to build (default: main)
#   DAYDREAM_REPO         the repository (default: https://github.com/jnfisher/daydream.git)
#   DAYDREAM_RESET_REALM  1: throw the realm away and convert the stock world again
#   DAYDREAM_NEW_TOKEN    1: make a new access token (every browser and connector then needs it)
#   DAYDREAM_SSH_HARDEN   0: leave sshd alone (default 1: key-only, when root has a key)
set -euo pipefail

say() { printf '\n==> %s\n' "$*"; }
die() { printf 'daydream setup: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "run as root (sudo bash $0)"
[ -r /etc/os-release ] && . /etc/os-release
case "${ID:-}" in ubuntu|debian) ;; *) die "this script knows Ubuntu and Debian only" ;; esac

if [ -r /etc/daydream/deploy.env ]; then
    set -a; . /etc/daydream/deploy.env; set +a
fi
REPO=${DAYDREAM_REPO:-https://github.com/jnfisher/daydream.git}
REF=${DAYDREAM_REF:-main}
HOME_DIR=/var/lib/daydream
SRC=$HOME_DIR/src
REALM=$HOME_DIR/realm
LISTEN=127.0.0.1:7340

public_ip() {
    ip -4 route get 1.1.1.1 2>/dev/null | sed -n 's/.* src \([0-9.]*\).*/\1/p' | head -n1
}
DOMAIN=${DAYDREAM_DOMAIN:-}
if [ -z "$DOMAIN" ]; then
    addr=$(public_ip)
    [ -n "$addr" ] || die "could not find this machine's IPv4 address; set DAYDREAM_DOMAIN"
    DOMAIN="${addr//./-}.sslip.io"
fi

say "Packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q build-essential pkg-config git curl ca-certificates ufw caddy

mem_kb=$(sed -n 's/^MemTotal: *\([0-9]*\) kB/\1/p' /proc/meminfo)
if [ "$mem_kb" -lt 3000000 ] && ! swapon --show=NAME --noheadings | grep -q .; then
    say "Swap: 2 GB at /swapfile (this machine has $((mem_kb / 1024)) MB of memory; the build needs more)"
    fallocate -l 2G /swapfile
    chmod 600 /swapfile
    mkswap /swapfile >/dev/null
    swapon /swapfile
    grep -q '^/swapfile ' /etc/fstab || echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

say "The daydream user"
if ! id daydream >/dev/null 2>&1; then
    useradd --system --user-group --create-home --home-dir "$HOME_DIR" --shell /usr/sbin/nologin daydream
fi
install -d -o daydream -g daydream -m 750 "$HOME_DIR"
as_dd() { runuser -u daydream -- env HOME="$HOME_DIR" PATH="$HOME_DIR/.cargo/bin:/usr/local/bin:/usr/bin:/bin" "$@"; }

say "Rust (rustup, as daydream; the repository pins the toolchain)"
if [ ! -x "$HOME_DIR/.cargo/bin/rustup" ]; then
    curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh
    as_dd sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain none --no-modify-path
    rm -f /tmp/rustup-init.sh
fi

say "Source: $REPO at $REF"
if [ ! -d "$SRC/.git" ]; then
    as_dd git clone --quiet "$REPO" "$SRC"
fi
as_dd git -C "$SRC" fetch --quiet origin "$REF"
as_dd git -C "$SRC" checkout --quiet --detach FETCH_HEAD
as_dd git -C "$SRC" log -1 --format='    %h %s'

say "Build (a few minutes the first time)"
as_dd sh -c "cd '$SRC' && cargo build --release --locked -p daydream"
install -m 755 "$SRC/target/release/daydream" /usr/local/bin/daydream
if ! /usr/local/bin/daydream new-token >/dev/null 2>&1; then
    die "$REF has no access token (\`daydream new-token\`), so it cannot be put on the internet. \
Build a ref that has it (the access-token pull request, or main once it is merged)."
fi

say "The realm: the stock 1.11p world"
if [ "${DAYDREAM_RESET_REALM:-0}" = 1 ] && [ -d "$REALM" ]; then
    systemctl stop daydream 2>/dev/null || true
    mv "$REALM" "$REALM.old-$(date -u +%Y%m%d-%H%M%S)"
fi
if [ ! -d "$REALM" ]; then
    as_dd daydream convert --stock --realm "$REALM"
else
    echo "    keeping $REALM (DAYDREAM_RESET_REALM=1 starts it over)"
fi

say "The access token"
install -d -m 755 /etc/daydream
if [ "${DAYDREAM_NEW_TOKEN:-0}" = 1 ] || ! grep -q '^DAYDREAM_TOKEN=' /etc/daydream/env 2>/dev/null; then
    umask 077
    printf 'DAYDREAM_TOKEN=%s\n' "$(/usr/local/bin/daydream new-token)" > /etc/daydream/env
    umask 022
    echo "    wrote a new one to /etc/daydream/env"
else
    echo "    keeping the one in /etc/daydream/env (DAYDREAM_NEW_TOKEN=1 makes a new one)"
fi
chmod 600 /etc/daydream/env

say "The service (daydream serve on $LISTEN)"
cat > /etc/systemd/system/daydream.service <<UNIT
[Unit]
Description=Daydream, the MajorMUD realm editor (test server)
After=network-online.target
Wants=network-online.target

[Service]
User=daydream
Group=daydream
EnvironmentFile=/etc/daydream/env
Environment=DAYDREAM_LISTEN=$LISTEN
ExecStart=/usr/local/bin/daydream serve $REALM
Restart=on-failure
RestartSec=5
UMask=0077
NoNewPrivileges=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$REALM
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictSUIDSGID=yes
LockPersonality=yes

[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable --quiet daydream
systemctl restart daydream

say "Caddy: https://$DOMAIN -> $LISTEN"
if [ -f /etc/caddy/Caddyfile ] && ! grep -q '^# Written by Daydream' /etc/caddy/Caddyfile; then
    cp /etc/caddy/Caddyfile "/etc/caddy/Caddyfile.before-daydream"
fi
cat > /etc/caddy/Caddyfile <<CADDY
# Written by Daydream's tools/deploy/setup.sh; running it again rewrites this file.
# No access log: an MCP connector's URL (/mcp/<token>) carries the token.
$DOMAIN {
	encode zstd gzip
	header Strict-Transport-Security "max-age=31536000"
	reverse_proxy $LISTEN
}
CADDY
caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null
systemctl enable --quiet caddy
systemctl reload-or-restart caddy

say "Firewall (ufw): SSH, 80 and 443"
ufw allow OpenSSH >/dev/null
ufw allow 80/tcp >/dev/null
ufw allow 443/tcp >/dev/null
ufw --force enable >/dev/null
ufw status | sed 's/^/    /'

if [ "${DAYDREAM_SSH_HARDEN:-1}" = 1 ]; then
    if [ -s /root/.ssh/authorized_keys ] || ls /home/*/.ssh/authorized_keys >/dev/null 2>&1; then
        say "SSH: keys only"
        cat > /etc/ssh/sshd_config.d/00-daydream.conf <<SSHD
# Written by Daydream's tools/deploy/setup.sh (DAYDREAM_SSH_HARDEN=0 skips it).
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin prohibit-password
SSHD
        if sshd -t; then
            systemctl reload ssh.service 2>/dev/null || systemctl reload sshd.service 2>/dev/null || true
        else
            rm -f /etc/ssh/sshd_config.d/00-daydream.conf
            echo "    sshd rejected the change; left SSH as it was"
        fi
    else
        say "SSH: left as it is (no authorized key found; add one, then run this again)"
    fi
fi

say "Checks"
for i in $(seq 1 30); do
    curl -fsS "http://$LISTEN/api/health" >/dev/null 2>&1 && break
    sleep 1
done
curl -fsS "http://$LISTEN/api/health" >/dev/null || die "daydream is not answering; journalctl -u daydream"
echo "    daydream answers on $LISTEN"
code=$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: example.com' "http://$LISTEN/api/realm" || true)
echo "    without the token: HTTP $code (401 expected)"
for i in $(seq 1 60); do
    curl -fsS "https://$DOMAIN/api/health" >/dev/null 2>&1 && break
    sleep 2
done
if curl -fsS "https://$DOMAIN/api/health" >/dev/null 2>&1; then
    echo "    https://$DOMAIN/ answers, with a certificate"
else
    echo "    https://$DOMAIN/ does not answer yet: check that its DNS points here, that ports 80"
    echo "    and 443 are open (the Vultr firewall group too), and journalctl -u caddy"
fi

cat <<DONE

Daydream is up.
  Browser:   https://$DOMAIN/   (sign in with the token)
  MCP:       https://$DOMAIN/mcp   with  Authorization: Bearer <token>
             https://$DOMAIN/mcp/<token>   for connectors that take only a URL
  The token: sudo grep DAYDREAM_TOKEN /etc/daydream/env   (on this machine; it is a password)
  Logs:      journalctl -u daydream -u caddy
  Update:    sudo bash $SRC/tools/deploy/setup.sh   (DAYDREAM_REF=... for another branch)
DONE
