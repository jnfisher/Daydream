#!/usr/bin/env bash
# Run a Daydream test server on this Linux machine and put it on the internet with
# Tailscale Funnel: build daydream from source, convert the stock MajorMUD 1.11p world into
# a realm, run `daydream serve` on 127.0.0.1 with an access token as a systemd user
# service, and funnel it at https://<this machine>.<tailnet>.ts.net. See docs/deploy-funnel.md.
#
# Run as yourself, not root. It asks for sudo only where it must (lingering, and
# `tailscale funnel` when you are not the tailscale operator). Safe to run again: it updates
# the source and the binary, keeps the realm and the token.
#
# Settings, from the environment or ~/.config/daydream/deploy.env:
#   DAYDREAM_HOME         where the source and realm live (default ~/.local/share/daydream)
#   DAYDREAM_REF          the branch, tag or commit to build (default: main)
#   DAYDREAM_REPO         the repository (default: https://github.com/jnfisher/daydream.git)
#   DAYDREAM_FUNNEL_PORT  the public HTTPS port: 443 (default), 8443 or 10000
#   DAYDREAM_PORT         the local port daydream listens on (default 7340)
#   DAYDREAM_RESET_REALM  1: move the realm aside and convert the stock world again
#   DAYDREAM_NEW_TOKEN    1: make a new access token (every browser and connector then needs it)
#
#   DAYDREAM_UPDATE_EVERY how often `auto-update on` checks (default 15min, systemd time span)
#
#   funnel.sh                 set up, or redo the whole setup
#   funnel.sh update          pull DAYDREAM_REF; when it moved, rebuild and restart (quick no-op
#                             otherwise). Keeps the realm, token, port and funnel.
#   funnel.sh auto-update on  run `update` on a timer (systemd user timer); `off` stops it
#   funnel.sh off             stop the funnel, the service and auto-update (realm and token stay)
#
# Setup installs this script as ~/.local/bin/daydream-funnel, so later it is just
# `daydream-funnel update`.
set -euo pipefail

say() { printf '\n==> %s\n' "$*"; }
die() { printf 'daydream funnel: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -ne 0 ] || die "run this as yourself, not root: the service runs as your user"

CONF=${XDG_CONFIG_HOME:-$HOME/.config}/daydream
if [ -r "$CONF/deploy.env" ]; then
    set -a; . "$CONF/deploy.env"; set +a
fi
DD_HOME=${DAYDREAM_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/daydream}
SRC=$DD_HOME/src
REALM=$DD_HOME/realm
BIN=$HOME/.local/bin/daydream
REPO=${DAYDREAM_REPO:-https://github.com/jnfisher/daydream.git}
REF=${DAYDREAM_REF:-main}
PORT=${DAYDREAM_PORT:-7340}
LISTEN=127.0.0.1:$PORT
FPORT=${DAYDREAM_FUNNEL_PORT:-443}
UNIT_DIR=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user
case "$FPORT" in 443|8443|10000) ;; *) die "DAYDREAM_FUNNEL_PORT must be 443, 8443 or 10000 (Funnel's ports)" ;; esac

# `tailscale funnel`, with sudo unless this user is tailscale's operator.
ts_funnel() {
    if tailscale debug prefs 2>/dev/null | grep -q "\"OperatorUser\": *\"$USER\""; then
        tailscale funnel "$@"
    else
        echo "    (sudo for tailscale; \`sudo tailscale set --operator=$USER\` avoids it next time)"
        sudo tailscale funnel "$@"
    fi
}

export PATH="$HOME/.cargo/bin:$PATH"
SELF=$HOME/.local/bin/daydream-funnel

# Fetch DAYDREAM_REF into the checkout and check it out.
fetch_source() {
    mkdir -p "$DD_HOME"
    [ -d "$SRC/.git" ] || git clone --quiet "$REPO" "$SRC"
    git -C "$SRC" fetch --quiet origin "$REF"
    git -C "$SRC" checkout --quiet --detach FETCH_HEAD
    git -C "$SRC" log -1 --format='    %h %s'
}

# Build and install the binary (and this script). Sets BIN_CHANGED=1 when the binary differs.
build_install() {
    (cd "$SRC" && cargo build --release --locked -p daydream)
    mkdir -p "$(dirname "$BIN")"
    BIN_CHANGED=0
    cmp -s "$SRC/target/release/daydream" "$BIN" || BIN_CHANGED=1
    "$SRC/target/release/daydream" new-token >/dev/null 2>&1 ||
        die "$REF has no access token (\`daydream new-token\`), so it cannot go on the internet"
    install -m 755 "$SRC/target/release/daydream" "$BIN.new" && mv -f "$BIN.new" "$BIN"
    # A copy then a rename, so a running daydream-funnel keeps reading its own file.
    local script="$SRC/tools/deploy/funnel.sh"
    [ -f "$script" ] || script=$(readlink -f "$0")
    if [ "$script" != "$(readlink -f "$SELF" 2>/dev/null)" ]; then
        install -m 755 "$script" "$SELF.new" && mv -f "$SELF.new" "$SELF"
    fi
}

wait_healthy() {
    for _ in $(seq 1 30); do
        curl -fsS "http://$LISTEN/api/health" >/dev/null 2>&1 && break
        sleep 1
    done
    curl -fsS "http://$LISTEN/api/health" >/dev/null || die "daydream is not answering; journalctl --user -u daydream"
    echo "    daydream answers on $LISTEN"
}

case "${1:-}" in
"" | setup) ;;
update)
    [ -f "$UNIT_DIR/daydream.service" ] && [ -f "$CONF/env" ] ||
        die "nothing to update yet: run funnel.sh once to set it up"
    before=$(git -C "$SRC" rev-parse HEAD 2>/dev/null || echo none)
    say "Update: $REPO at $REF"
    fetch_source
    after=$(git -C "$SRC" rev-parse HEAD)
    if [ "$before" = "$after" ] && [ -x "$BIN" ] && [ "${DAYDREAM_FORCE:-0}" != 1 ]; then
        echo "    already up to date (DAYDREAM_FORCE=1 rebuilds anyway)"
        exit 0
    fi
    [ "$before" = none ] || git -C "$SRC" log --oneline --no-decorate "$before..$after" 2>/dev/null | head -n 20 | sed 's/^/    /'
    say "Build"
    build_install
    if [ "$BIN_CHANGED" = 1 ] || ! systemctl --user is-active --quiet daydream; then
        systemctl --user restart daydream
        wait_healthy
    else
        echo "    the binary did not change; daydream keeps running"
    fi
    exit 0
    ;;
auto-update)
    case "${2:-}" in
    on)
        [ -x "$SELF" ] || die "run funnel.sh once to set it up first"
        mkdir -p "$UNIT_DIR"
        cat > "$UNIT_DIR/daydream-update.service" <<UNIT
# Written by Daydream's tools/deploy/funnel.sh.
[Unit]
Description=Update Daydream from $REPO ($REF)

[Service]
Type=oneshot
Environment=PATH=$HOME/.cargo/bin:$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=$SELF update
UNIT
        cat > "$UNIT_DIR/daydream-update.timer" <<UNIT
# Written by Daydream's tools/deploy/funnel.sh.
[Unit]
Description=Update Daydream every ${DAYDREAM_UPDATE_EVERY:-15min}

[Timer]
OnBootSec=5min
OnUnitActiveSec=${DAYDREAM_UPDATE_EVERY:-15min}

[Install]
WantedBy=timers.target
UNIT
        systemctl --user daemon-reload
        systemctl --user enable --now --quiet daydream-update.timer
        echo "auto-update on: every ${DAYDREAM_UPDATE_EVERY:-15min} from $REF"
        echo "  last runs: journalctl --user -u daydream-update    next: systemctl --user list-timers daydream-update"
        ;;
    off)
        systemctl --user disable --now --quiet daydream-update.timer 2>/dev/null || true
        echo "auto-update off"
        ;;
    *) die "auto-update on, or auto-update off" ;;
    esac
    exit 0
    ;;
off)
    say "Stopping the funnel on $FPORT, the service and auto-update"
    ts_funnel --https="$FPORT" off || true
    systemctl --user disable --now --quiet daydream-update.timer 2>/dev/null || true
    systemctl --user disable --now daydream 2>/dev/null || true
    echo "    stopped; the realm ($REALM) and the token ($CONF/env) are kept"
    exit 0
    ;;
*) die "unknown command ${1}: funnel.sh [update | auto-update on|off | off]" ;;
esac

say "Checks"
for cmd in git curl cc tailscale systemctl; do
    command -v "$cmd" >/dev/null || die "needs $cmd (on Debian/Ubuntu: sudo apt install git curl build-essential; Tailscale from tailscale.com/download)"
done
ts_json=$(tailscale status --json 2>/dev/null) || die "tailscale is not running or not logged in (tailscale up)"
DNS=$(printf '%s' "$ts_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["Self"]["DNSName"])' 2>/dev/null ||
      printf '%s' "$ts_json" | sed -n 's/.*"DNSName": *"\([^"]*\)".*/\1/p' | head -n1)
DNS=${DNS%.}
[ -n "$DNS" ] || die "could not read this machine's tailnet name; is MagicDNS on (admin console, DNS)?"
URL="https://$DNS"; [ "$FPORT" = 443 ] || URL="$URL:$FPORT"
echo "    this machine is $DNS"
# Another `tailscale serve` or funnel on the same port would be replaced: refuse instead.
if [ "$FPORT" = 443 ]; then mine="https://$DNS "; else mine="https://$DNS:$FPORT "; fi
serve_now=$(tailscale serve status 2>/dev/null || true)
if printf '%s\n' "$serve_now" | grep -qF "$mine" && ! printf '%s' "$serve_now" | grep -qF "127.0.0.1:$PORT"; then
    die "tailscale already serves something at ${mine% } (tailscale serve status). \
Pick another DAYDREAM_FUNNEL_PORT: 443, 8443 or 10000"
fi

# Remember the settings this setup ran with, for `update`, `off` and the timer.
mkdir -p "$CONF"; chmod 700 "$CONF"
touch "$CONF/deploy.env"
for kv in "DAYDREAM_REF=$REF" "DAYDREAM_FUNNEL_PORT=$FPORT"; do
    sed -i "/^${kv%%=*}=/d" "$CONF/deploy.env"
    printf '%s\n' "$kv" >> "$CONF/deploy.env"
done

say "Rust"
if ! command -v rustup >/dev/null; then
    echo "    installing rustup for $USER (~/.cargo; the repository pins the toolchain)"
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none --no-modify-path
fi

say "Source: $REPO at $REF"
fetch_source

say "Build"
build_install

say "The realm: the stock 1.11p world"
if [ "${DAYDREAM_RESET_REALM:-0}" = 1 ] && [ -d "$REALM" ]; then
    systemctl --user stop daydream 2>/dev/null || true
    mv "$REALM" "$REALM.old-$(date -u +%Y%m%d-%H%M%S)"
fi
if [ -d "$REALM" ]; then
    echo "    keeping $REALM (DAYDREAM_RESET_REALM=1 starts it over)"
else
    "$BIN" convert --stock --realm "$REALM"
fi

say "The access token"
mkdir -p "$CONF"; chmod 700 "$CONF"
if [ "${DAYDREAM_NEW_TOKEN:-0}" = 1 ] || ! grep -q '^DAYDREAM_TOKEN=' "$CONF/env" 2>/dev/null; then
    (umask 077; printf 'DAYDREAM_TOKEN=%s\n' "$("$BIN" new-token)" > "$CONF/env")
    echo "    wrote a new one to $CONF/env"
else
    echo "    keeping the one in $CONF/env (DAYDREAM_NEW_TOKEN=1 makes a new one)"
fi
chmod 600 "$CONF/env"

say "The service: daydream serve on $LISTEN (systemd user unit)"
mkdir -p "$UNIT_DIR"
cat > "$UNIT_DIR/daydream.service" <<UNIT
# Written by Daydream's tools/deploy/funnel.sh; running it again rewrites this file.
[Unit]
Description=Daydream, the MajorMUD realm editor (test server)

[Service]
EnvironmentFile=$CONF/env
Environment=DAYDREAM_LISTEN=$LISTEN
ExecStart=$BIN serve $REALM
Restart=on-failure
RestartSec=5
UMask=0077

[Install]
WantedBy=default.target
UNIT
systemctl --user daemon-reload
systemctl --user enable --quiet daydream
systemctl --user restart daydream
wait_healthy
if [ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null)" != yes ]; then
    echo "    keeping it running after you log out and across reboots needs lingering:"
    sudo loginctl enable-linger "$USER" || echo "    run: sudo loginctl enable-linger $USER"
fi

say "Tailscale Funnel: $URL -> $LISTEN"
echo "    the first time, tailscale may print a link to allow Funnel for this machine: open it, then come back"
ts_funnel --bg --https="$FPORT" "http://$LISTEN"

say "Checks"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/api/realm" || true)
echo "    $URL/api/realm without the token: HTTP $code (401 expected)"
echo "    (this machine reaches its own name over the tailnet; to see it from outside,"
echo "    open $URL on a phone with wifi off)"

cat <<DONE

Daydream is up.
  Browser:   $URL/   (sign in with the token)
  MCP:       $URL/mcp   with  Authorization: Bearer <token>
             $URL/mcp/<token>   for connectors that take only a URL
  The token: grep DAYDREAM_TOKEN $CONF/env   (a password for the realm)
  Logs:      journalctl --user -u daydream
  Update:    daydream-funnel update   (or: daydream-funnel auto-update on)
  Stop:      daydream-funnel off
DONE
