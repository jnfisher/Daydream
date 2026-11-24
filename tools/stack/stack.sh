#!/usr/bin/env bash
# daydream-stack: everything needed to check a Daydream edit in the game, on one Linux or macOS
# machine. silvermere (the game server, from its GitHub release) with a seeded test character,
# Daydream (from its release, else built from source), and SyncTERM logged in as that character.
# See docs/stack.md.
#
#   stack.sh init               install everything, seed the character, start, open SyncTERM
#   daydream-stack up [C]       start silvermere and Daydream (or one: silvermere, daydream)
#   daydream-stack down [C]     stop them; silvermere saves every character first
#   daydream-stack connect      open SyncTERM logged in as the test character
#   daydream-stack edit         stop silvermere so Daydream can write its database
#   daydream-stack play         start silvermere again and reopen SyncTERM
#   daydream-stack seed         make the test character again (level, stats, gear)
#   daydream-stack update [C] [--version V]   newest (or V) release of silvermere, daydream,
#                               or syncterm (the package manager's); --rollback goes back one
#   daydream-stack reset        back up, then the stock world again and a fresh character
#   daydream-stack status       versions, running state, addresses and the login
#   daydream-stack logs C       follow silvermere's or daydream's log
#   daydream-stack remove       stop and remove the services (the data stays)
#
# Settings, from the environment or ~/.config/daydream-stack/stack.env (written by init, mode
# 600; the environment wins for a run):
#   DAYDREAM_STACK_HOME       where everything lives (default ~/.local/share/daydream-stack)
#   STACK_TELNET_PORT         silvermere's telnet port (2323)
#   STACK_ADMIN_PORT          silvermere's admin API port (8081)
#   STACK_DAYDREAM_PORT       Daydream's port (7350)
#   STACK_USER, STACK_PASSWORD    the test account (tester, a random password)
#   STACK_NAME, STACK_RACE, STACK_CLASS, STACK_LEVEL, STACK_GOLD   the character
#   STACK_ITEMS               its gear, item names separated by `;` (see DEFAULT_ITEMS)
#   STACK_SILVERMERE_REPO, STACK_DAYDREAM_REPO   GitHub owner/name of each release
#   STACK_SILVERMERE_BIN      a silvermere binary to use in place of the release
#   STACK_STOCK_ZIP           a saved WCCMMUD_1.11p_DOS_MBBSEmu_v2.zip, when the download is blocked
#
# Runs as yourself, never root. It never touches ~/silvermere or a daydream-funnel install.
set -euo pipefail

say() { printf '\n==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }
die() { printf 'daydream-stack: %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -ne 0 ] || die "run this as yourself, not root"
OS=$(uname -s)
case "$OS" in Linux | Darwin) ;; *) die "this script knows Linux and macOS only" ;; esac

CONF_DIR=${XDG_CONFIG_HOME:-$HOME/.config}/daydream-stack
CONF=$CONF_DIR/stack.env
# Saved settings, under any given in the environment for this run.
if [ -r "$CONF" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
        STACK_*=* | DAYDREAM_STACK_*=*)
            k=${line%%=*}
            if [ -z "${!k+x}" ]; then export "${line?}"; fi
            ;;
        esac
    done < "$CONF"
fi

# Stock 1.11p items a Warrior can use, none of them limited: the best armour for each place,
# a one-handed hammer and a shield. An item the class cannot use is carried, not worn, and
# `seed` says so.
# Checked against the stock 1.11p world: every piece is worn or wielded by a Dwarf Warrior,
# and with the gold the pack is at 4201 of 6768.
DEFAULT_ITEMS="stormhammer;aegis shield;mithril chainmail hauberk;starsteel greathelm;starsteel plate gauntlets;starsteel plate boots;starsteel plate leggings;stormmetal bracers;hero's tabard;glowing red amulet;belt of the crimson flame;sapphire ring;moonstone ring"

HOME_DIR=${DAYDREAM_STACK_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/daydream-stack}
SM=$HOME_DIR/silvermere
DD=$HOME_DIR/daydream
TELNET_PORT=${STACK_TELNET_PORT:-2323}
ADMIN_PORT=${STACK_ADMIN_PORT:-8081}
DD_PORT=${STACK_DAYDREAM_PORT:-7350}
USER_NAME=${STACK_USER:-tester}
CHAR_NAME=${STACK_NAME:-Tester}
RACE=${STACK_RACE:-Dwarf}  # a Dwarf sees in the dark
CLASS=${STACK_CLASS:-Warrior}
LEVEL=${STACK_LEVEL:-70}
GOLD=${STACK_GOLD:-500}  # coins weigh: three to a unit
ITEMS=${STACK_ITEMS:-$DEFAULT_ITEMS}
SM_REPO=${STACK_SILVERMERE_REPO:-jnfisher/silvermere}
DD_REPO=${STACK_DAYDREAM_REPO:-jnfisher/daydream}
SELF=$HOME/.local/bin/daydream-stack
ENTRY=daydream-stack

# --- small helpers ------------------------------------------------------------------------

# The absolute path of a file, without `readlink -f` (older macOS has none).
abspath() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$(pwd -P)" "$(basename "$1")"); }
# Text going into a launchd plist needs &, < and > escaped.
xesc() { printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
sha256() { if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi; }
port_open() { (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; }
random_password() { LC_ALL=C tr -dc 'a-km-zA-HJ-NP-Z2-9' < /dev/urandom | head -c 16; }

# Writes one KEY=value into stack.env (mode 600), replacing an older line.
save_setting() {
    mkdir -p "$CONF_DIR"
    (umask 077 && touch "$CONF")
    local tmp=$CONF.new
    grep -v "^$1=" "$CONF" > "$tmp" || true
    printf '%s=%s\n' "$1" "$2" >> "$tmp"
    mv -f "$tmp" "$CONF"
    chmod 600 "$CONF"
    export "$1=$2"
}

# The release archive's target for this machine, as silvermere and daydream name them.
target() {
    case "$OS/$(uname -m)" in
    Linux/x86_64 | Linux/amd64) echo x86_64-unknown-linux-musl ;;
    Linux/aarch64 | Linux/arm64) echo aarch64-unknown-linux-musl ;;
    Darwin/*) echo universal-apple-darwin ;;
    *) die "no release build for $OS $(uname -m)" ;;
    esac
}

# The newest release tag of a GitHub repository (from the /releases/latest redirect, which
# needs no API token), or nothing when it has no release.
latest_tag() {
    curl -fsSI "https://github.com/$1/releases/latest" 2>/dev/null |
        tr -d '\r' | sed -n 's|^[Ll]ocation: .*/releases/tag/\(.*\)$|\1|p' | tail -n1
}

# Downloads NAME's release VERSION (vX.Y.Z) from REPO into DIR/releases/X.Y.Z, checks it
# against the release's SHA256SUMS, and points DIR/releases/current at it.
install_release() {
    local name=$1 repo=$2 dir=$3 tag=$4
    local v=${tag#v} t file tmp
    t=$(target)
    file="$name-$v-$t.tar.gz"
    if [ -x "$dir/releases/$v/$name" ]; then
        note "$name $v is already here"
    else
        tmp=$(mktemp -d)
        note "downloading $file"
        curl -fsSL --retry 3 -o "$tmp/$file" "https://github.com/$repo/releases/download/$tag/$file" ||
            { rm -rf "$tmp"; die "could not download $name $tag for $t"; }
        curl -fsSL --retry 3 -o "$tmp/SHA256SUMS" "https://github.com/$repo/releases/download/$tag/SHA256SUMS" ||
            { rm -rf "$tmp"; die "could not download $name $tag's SHA256SUMS"; }
        (cd "$tmp" && grep " $file\$" SHA256SUMS | sha256 -c - >/dev/null) ||
            { rm -rf "$tmp"; die "$file does not match the release's SHA256SUMS"; }
        tar -xzf "$tmp/$file" -C "$tmp"
        mkdir -p "$dir/releases/$v"
        cp "$tmp/$name-$v-$t/$name" "$dir/releases/$v/$name"
        chmod 755 "$dir/releases/$v/$name"
        rm -rf "$tmp"
        note "checked and unpacked into $dir/releases/$v"
    fi
    switch_to "$dir" "$v"
}

# Points DIR/releases/current at VERSION, keeping the one before as previous.
switch_to() {
    local dir=$1 v=$2 cur
    cur=$(readlink "$dir/releases/current" 2>/dev/null || true)
    if [ -n "$cur" ] && [ "$cur" != "$v" ]; then ln -sfn "$cur" "$dir/releases/previous"; fi
    ln -sfn "$v" "$dir/releases/current"
}

rollback() {
    local dir=$1 prev
    prev=$(readlink "$dir/releases/previous" 2>/dev/null) || die "no previous version in $dir/releases"
    switch_to "$dir" "$prev"
    note "back to $prev"
}

SMBIN=$SM/releases/current/silvermere
version_of() { readlink "$1/releases/current" 2>/dev/null || echo "not installed"; }

# silvermere, run in its folder (its silvermere.toml and data/ are there).
sm() { (cd "$SM" && "$SMBIN" --log-level warn "$@"); }

# --- services: systemd user units on Linux, launchd agents on macOS -----------------------

UNIT_DIR=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user
AGENTS=$HOME/Library/LaunchAgents
MACLOGS=$HOME/Library/Logs/daydream-stack
label() { echo "io.github.jnfisher.daydream-stack.$1"; }
unit() { echo "daydream-stack-$1"; }

# Writes the service for component C running COMMAND... in DIR, with KEY=VALUE environment
# lines from ENV (newline separated). A clean exit (silvermere's planned shutdown, SIGTERM)
# leaves it stopped; a crash restarts it.
svc_write() {
    local c=$1 dir=$2 env=$3
    shift 3
    if [ "$OS" = Darwin ]; then
        mkdir -p "$AGENTS" "$MACLOGS"
        {
            echo '<?xml version="1.0" encoding="UTF-8"?>'
            echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">'
            echo '<!-- Written by daydream-stack (Daydream tools/stack). -->'
            echo '<plist version="1.0"><dict>'
            echo "    <key>Label</key><string>$(label "$c")</string>"
            echo '    <key>ProgramArguments</key><array>'
            for a in "$@"; do echo "        <string>$(xesc "$a")</string>"; done
            echo '    </array>'
            echo "    <key>WorkingDirectory</key><string>$(xesc "$dir")</string>"
            echo '    <key>EnvironmentVariables</key><dict>'
            while IFS= read -r kv; do
                [ -n "$kv" ] || continue
                echo "        <key>$(xesc "${kv%%=*}")</key><string>$(xesc "${kv#*=}")</string>"
            done <<< "$env"
            echo '    </dict>'
            # without RunAtLoad, a SuccessfulExit KeepAlive waits for an exit that never comes
            echo '    <key>RunAtLoad</key><true/>'
            echo '    <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>'
            # silvermere saves every character on SIGTERM; give it the time it takes
            echo '    <key>ExitTimeOut</key><integer>90</integer>'
            echo "    <key>StandardOutPath</key><string>$(xesc "$MACLOGS/$c.log")</string>"
            echo "    <key>StandardErrorPath</key><string>$(xesc "$MACLOGS/$c.log")</string>"
            echo '</dict></plist>'
        } > "$AGENTS/$(label "$c").plist"
    else
        mkdir -p "$UNIT_DIR"
        {
            echo "# Written by daydream-stack (Daydream tools/stack)."
            echo "[Unit]"
            echo "Description=daydream-stack: $c"
            echo
            echo "[Service]"
            echo "WorkingDirectory=$dir"
            while IFS= read -r kv; do
                [ -n "$kv" ] || continue
                echo "Environment=\"$kv\""
            done <<< "$env"
            printf 'ExecStart='
            printf '"%s" ' "$@"
            echo
            echo "Restart=on-failure"
            echo "RestartSec=5"
            # silvermere saves every character on SIGTERM; give it the time it takes
            echo "TimeoutStopSec=90"
            echo
            echo "[Install]"
            echo "WantedBy=default.target"
        } > "$UNIT_DIR/$(unit "$c").service"
        systemctl --user daemon-reload
    fi
}
svc_exists() {
    if [ "$OS" = Darwin ]; then [ -f "$AGENTS/$(label "$1").plist" ]; else [ -f "$UNIT_DIR/$(unit "$1").service" ]; fi
}
svc_active() {
    if [ "$OS" = Darwin ]; then
        launchctl print "gui/$UID/$(label "$1")" 2>/dev/null | grep -q 'state = running'
    else
        systemctl --user is-active --quiet "$(unit "$1")" 2>/dev/null
    fi
}
svc_start() {
    if [ "$OS" = Darwin ]; then
        launchctl bootout "gui/$UID/$(label "$1")" 2>/dev/null || true
        launchctl bootstrap "gui/$UID" "$AGENTS/$(label "$1").plist"
        # launchd may hold a RunAtLoad job as a "speculative" spawn and start it later, or not at
        # all: start it now
        launchctl kickstart "gui/$UID/$(label "$1")"
    else
        systemctl --user enable --quiet "$(unit "$1")"
        systemctl --user restart "$(unit "$1")"
    fi
}
svc_stop() {
    if [ "$OS" = Darwin ]; then
        launchctl bootout "gui/$UID/$(label "$1")" 2>/dev/null || true
    else
        systemctl --user stop "$(unit "$1")" 2>/dev/null || true
    fi
}
svc_remove() {
    svc_stop "$1"
    if [ "$OS" = Darwin ]; then
        rm -f "${AGENTS:?}/$(label "$1").plist"
    else
        systemctl --user disable --quiet "$(unit "$1")" 2>/dev/null || true
        rm -f "${UNIT_DIR:?}/$(unit "$1").service"
        systemctl --user daemon-reload
    fi
}
logs_cmd() {
    if [ "$OS" = Darwin ]; then
        # launchd writes nothing to the log when it never started the program: say what it knows
        if [ ! -s "$MACLOGS/$1.log" ]; then
            launchctl print "gui/$UID/$(label "$1")" 2>&1 |
                grep -iE 'state =|last exit|spawn|error' | sed 's/^[[:space:]]*/    launchd: /' >&2 || true
            launchctl print-disabled "gui/$UID" 2>/dev/null | grep -qE "\"$(label "$1")\" => (true|disabled)" &&
                echo "    launchd: the job is disabled (System Settings > General > Login Items & Extensions)" >&2
        fi
        echo "tail -f $MACLOGS/$1.log"
    else
        echo "journalctl --user -u $(unit "$1") -f"
    fi
}

# Waits up to $2 seconds for PORT to answer (or, with "closed", to stop answering).
wait_port() {
    local port=$1 secs=$2 want=${3:-open}
    for _ in $(seq 1 "$secs"); do
        if [ "$want" = open ]; then port_open "$port" && return 0; else port_open "$port" || return 0; fi
        sleep 1
    done
    return 1
}

# --- the components -----------------------------------------------------------------------

write_silvermere_config() {
    mkdir -p "$SM/data"
    cat > "$SM/silvermere.toml" <<TOML
# Written by daydream-stack: silvermere for in-game checks of Daydream edits.
[server]
telnet_bind = "127.0.0.1:$TELNET_PORT"
admin_bind = "127.0.0.1:$ADMIN_PORT"
web_bind = ""
TOML
}

silvermere_service() {
    svc_write silvermere "$SM" "" "$(abspath "$SM")/releases/current/silvermere" serve
}

daydream_service() {
    local env="DAYDREAM_HOME=$DD/home
DAYDREAM_STOCK_CACHE=$SM/data/cache"
    svc_write daydream "$DD" "$env" "$(abspath "$DD")/releases/current/daydream" serve \
        --listen "127.0.0.1:$DD_PORT" --search "$SM" --silvermere "$SM/data/silvermere.db"
}

# MajorMUD 1.11p's stock files, which `silvermere setup` downloads once into its cache.
STOCK_ZIP=WCCMMUD_1.11p_DOS_MBBSEmu_v2.zip
STOCK_SHA256=815f7b35cdbc6c4de8deb0b686031e34905da147f360655858058a1f642d3583

# Puts the stock zip at PATH into silvermere's cache, if it is the right file.
cache_stock_zip() {
    local zip=$1
    zip=${zip/#\~/$HOME}
    [ -f "$zip" ] || { note "no file at $zip"; return 1; }
    if [ "$(sha256 "$zip" | awk '{print $1}')" != "$STOCK_SHA256" ]; then
        note "$zip is not $STOCK_ZIP (its checksum differs)"
        return 1
    fi
    mkdir -p "$SM/data/cache"
    cp "$zip" "$SM/data/cache/$STOCK_ZIP"
    note "using $zip"
}

# Builds the stock world. The download can be blocked (a company network that inspects
# HTTPS, for one): then a copy of the zip is taken from STACK_STOCK_ZIP, ~/Downloads, or
# a path typed at the prompt.
stock_world() {
    if [ ! -f "$SM/data/cache/$STOCK_ZIP" ]; then
        if [ -n "${STACK_STOCK_ZIP:-}" ]; then
            cache_stock_zip "$STACK_STOCK_ZIP" || die "STACK_STOCK_ZIP is not a usable $STOCK_ZIP"
        elif [ -f "$HOME/Downloads/$STOCK_ZIP" ]; then
            cache_stock_zip "$HOME/Downloads/$STOCK_ZIP" || true
        fi
    fi
    note "building the stock world (downloads MajorMUD 1.11p once)"
    while ! sm setup --yes < /dev/null 2>&1 | sed 's/^/    /'; do
        rm -f "$SM/data/cache/$STOCK_ZIP"
        if ! { [ -t 1 ] && : < /dev/tty; } 2>/dev/null; then
            die "the stock world could not be built. If its download is blocked, get $STOCK_ZIP another way and run again with STACK_STOCK_ZIP=/path/to/it"
        fi
        printf '\n    The stock download failed. If your network blocks it, download\n'
        printf '    %s another way and give its path here.\n' "$STOCK_ZIP"
        local path
        while :; do
            path=
            read -r -p "    Path to $STOCK_ZIP (Enter to give up): " path < /dev/tty || path=
            # a file dragged into Terminal arrives quoted or with escaped spaces, and a space after
            path=${path%"${path##*[![:space:]]}"}
            path=${path#\'}; path=${path%\'}; path=${path#\"}; path=${path%\"}
            path=${path//\\ / }
            [ -n "$path" ] || die "no stock world: run again with STACK_STOCK_ZIP=/path/to/$STOCK_ZIP"
            cache_stock_zip "$path" && break
        done
        save_setting STACK_STOCK_ZIP "${path/#\~/$HOME}"
        note "trying again"
    done
}

fetch_silvermere() {
    local tag=${1:-}
    if [ -n "${STACK_SILVERMERE_BIN:-}" ]; then
        # a silvermere of your own (a build not yet released) in place of the release
        local v
        v="local-$("$STACK_SILVERMERE_BIN" --version | awk '{print $2}')"
        mkdir -p "$SM/releases/$v"
        install -m 755 "$STACK_SILVERMERE_BIN" "$SM/releases/$v/silvermere"
        switch_to "$SM" "$v"
        note "silvermere from $STACK_SILVERMERE_BIN ($v)"
        return
    fi
    [ -n "$tag" ] || tag=$(latest_tag "$SM_REPO")
    [ -n "$tag" ] || die "could not find $SM_REPO's latest release"
    case "$tag" in v*) ;; *) tag=v$tag ;; esac
    install_release silvermere "$SM_REPO" "$SM" "$tag"
}

# Daydream's release, or a build from source while it has none.
fetch_daydream() {
    local tag=${1:-}
    [ -n "$tag" ] || tag=$(latest_tag "$DD_REPO")
    if [ -n "$tag" ]; then
        case "$tag" in v*) ;; *) tag=v$tag ;; esac
        install_release daydream "$DD_REPO" "$DD" "$tag"
        return
    fi
    note "$DD_REPO has no release yet: building it from source (needs git and cargo)"
    command -v cargo >/dev/null || export PATH="$HOME/.cargo/bin:$PATH"
    command -v git >/dev/null && command -v cargo >/dev/null ||
        die "building Daydream needs git and Rust (rustup.rs); or wait for a Daydream release"
    [ -d "$DD/src/.git" ] || git clone --quiet "https://github.com/$DD_REPO.git" "$DD/src"
    git -C "$DD/src" fetch --quiet origin main
    git -C "$DD/src" checkout --quiet --detach FETCH_HEAD
    (cd "$DD/src" && cargo build --release --locked -p daydream)
    local v
    v="src-$(git -C "$DD/src" rev-parse --short HEAD)"
    mkdir -p "$DD/releases/$v"
    install -m 755 "$DD/src/target/release/daydream" "$DD/releases/$v/daydream"
    switch_to "$DD" "$v"
}

# The test account (sysop, DEBUG) and its character, made again from the settings.
seed() {
    local was_up=0 args=()
    if svc_active silvermere; then was_up=1; down_silvermere; fi
    if has_account; then
        printf '%s\n' "$STACK_PASSWORD" | sm account passwd "$USER_NAME" >/dev/null 2>&1
    else
        printf '%s\n' "$STACK_PASSWORD" | sm account create "$USER_NAME" --sysop --password-stdin
    fi
    sm account grant "$USER_NAME" sysop >/dev/null 2>&1 || true
    sm account grant "$USER_NAME" DEBUG >/dev/null 2>&1 || true
    sm character --help >/dev/null 2>&1 ||
        die "this silvermere ($("$SMBIN" --version)) has no \`character create\`: it needs 1.0.2 or later (\`daydream-stack update silvermere\`, or STACK_SILVERMERE_BIN for a build of your own)"
    IFS=';' read -r -a list <<< "$ITEMS"
    for i in "${list[@]}"; do
        i=$(printf '%s' "$i" | sed 's/^ *//; s/ *$//')
        [ -n "$i" ] && args+=(--item "$i")
    done
    sm character create "$USER_NAME" --name "$CHAR_NAME" --race "$RACE" --class "$CLASS" \
        --level "$LEVEL" --gold "$GOLD" --replace "${args[@]}" | sed 's/^/    /'
    if [ "$was_up" = 1 ]; then up_silvermere; fi
}

up_silvermere() {
    svc_exists silvermere || die "not set up yet: run init"
    port_open "$TELNET_PORT" && ! svc_active silvermere &&
        die "something else answers on port $TELNET_PORT (STACK_TELNET_PORT picks another)"
    svc_active silvermere || svc_start silvermere
    wait_port "$TELNET_PORT" 60 || die "silvermere did not start; $(logs_cmd silvermere)"
    note "silvermere: telnet 127.0.0.1:$TELNET_PORT"
}

# SIGTERM: silvermere saves every character, then exits.
down_silvermere() {
    svc_active silvermere || { note "silvermere is not running"; return 0; }
    svc_stop silvermere
    wait_port "$TELNET_PORT" 90 closed || die "silvermere has not stopped; $(logs_cmd silvermere)"
    # the port closes first; the process (and its writer lock) goes once every character is saved
    local pid
    pid=$(tr -cd '0-9' < "$SM/data/silvermere.db.lock" 2>/dev/null || true)
    if [ -n "$pid" ]; then
        for _ in $(seq 1 90); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done
        ! kill -0 "$pid" 2>/dev/null || die "silvermere (pid $pid) has not exited; $(logs_cmd silvermere)"
    fi
    note "silvermere stopped, every character saved"
}

up_daydream() {
    svc_exists daydream || die "not set up yet: run init"
    svc_active daydream || svc_start daydream
    wait_port "$DD_PORT" 60 || die "Daydream did not start; $(logs_cmd daydream)"
    note "Daydream: http://127.0.0.1:$DD_PORT/"
}
down_daydream() { svc_stop daydream; note "Daydream stopped"; }

# --- SyncTERM ------------------------------------------------------------------------------

syncterm_list() {
    if [ "$OS" = Darwin ]; then echo "$HOME/Library/Preferences/SyncTERM/syncterm.lst"; else echo "$HOME/.syncterm/syncterm.lst"; fi
}
syncterm_found() {
    if [ "$OS" = Darwin ]; then
        [ -d /Applications/SyncTERM.app ] || [ -d "$HOME/Applications/SyncTERM.app" ] || command -v syncterm >/dev/null
    else
        command -v syncterm >/dev/null
    fi
}

install_syncterm() {
    if syncterm_found; then note "SyncTERM is installed"; return 0; fi
    if [ "$OS" = Darwin ]; then
        # Homebrew disabled its cask (2026-09-01: SyncTERM does not pass Gatekeeper)
        note "SyncTERM is not installed. Get the macOS build from https://syncterm.bbsdev.net/,"
        note "move it to Applications, run 'xattr -dr com.apple.quarantine /Applications/SyncTERM.app'"
        note "(Gatekeeper will not open it otherwise), then 'daydream-stack connect'."
        note "Until then 'daydream-stack connect' plays in this terminal with telnet (brew install telnet)."
        return 1
    elif command -v apt-get >/dev/null; then
        note "installing SyncTERM with apt (asks for sudo)"
        sudo apt-get install -y syncterm && return 0
    elif command -v dnf >/dev/null; then
        sudo dnf install -y syncterm && return 0
    fi
    note "could not install SyncTERM here: get it from https://syncterm.bbsdev.net/ and run 'daydream-stack connect'"
    return 1
}

# SyncTERM's dialing directory gets one entry of ours; every other entry stays as it was.
write_syncterm_entry() {
    local lst tmp
    lst=$(syncterm_list)
    mkdir -p "$(dirname "$lst")"
    (umask 077 && touch "$lst")
    tmp=$lst.new
    awk -v name="[$ENTRY]" '
        /^\[/ { skip = ($0 == name) }
        !skip { print }
    ' "$lst" > "$tmp"
    cat >> "$tmp" <<LST

[$ENTRY]
Address=127.0.0.1
Port=$TELNET_PORT
ConnectionType=Telnet
Username=$USER_NAME
Password=$STACK_PASSWORD
ScreenMode=80x25
LST
    mv -f "$tmp" "$lst"
    chmod 600 "$lst"
    note "SyncTERM entry '$ENTRY' in $lst"
}

connect() {
    port_open "$TELNET_PORT" || die "silvermere is not running: daydream-stack up"
    if [ -n "${STACK_CLIENT_CMD:-}" ]; then
        sh -c "$STACK_CLIENT_CMD" &
        return 0
    elif ! syncterm_found; then
        # no SyncTERM: play in this terminal
        command -v telnet >/dev/null ||
            die "SyncTERM is not installed (https://syncterm.bbsdev.net/), and neither is telnet (brew install telnet, or apt install telnet)"
        note "no SyncTERM: telnet in this terminal. Log in as $USER_NAME / ${STACK_PASSWORD:-?}; Ctrl-] then 'quit' leaves."
        telnet 127.0.0.1 "$TELNET_PORT" || true
        return 0
    fi
    # SyncTERM may have been installed after init: make sure our entry is there
    write_syncterm_entry
    if [ "$OS" = Darwin ] && ! command -v syncterm >/dev/null; then
        open -a SyncTERM --args "$ENTRY"
    else
        nohup syncterm "$ENTRY" >/dev/null 2>&1 &
    fi
    note "SyncTERM opened on '$ENTRY' (user $USER_NAME); if it stops at Username:, the login is in 'daydream-stack status'"
}

# --- commands -----------------------------------------------------------------------------

status() {
    printf 'silvermere %s: %s, telnet 127.0.0.1:%s\n' "$(version_of "$SM")" \
        "$(svc_active silvermere && echo running || echo stopped)" "$TELNET_PORT"
    printf 'daydream   %s: %s, http://127.0.0.1:%s/\n' "$(version_of "$DD")" \
        "$(svc_active daydream && echo running || echo stopped)" "$DD_PORT"
    printf 'syncterm   %s, entry %s\n' "$(syncterm_found && echo installed || echo 'not installed')" "$ENTRY"
    printf 'login      %s / %s (character %s, level %s %s %s)\n' "$USER_NAME" "${STACK_PASSWORD:-?}" \
        "$CHAR_NAME" "$LEVEL" "$RACE" "$CLASS"
    printf 'data       %s\n' "$HOME_DIR"
}

init() {
    say "Checks"
    for cmd in curl tar; do command -v "$cmd" >/dev/null || die "needs $cmd"; done
    if [ "$OS" = Linux ]; then command -v systemctl >/dev/null || die "needs systemd (systemctl --user)"; fi
    port_open "$TELNET_PORT" && ! svc_active silvermere 2>/dev/null &&
        die "port $TELNET_PORT is taken (another silvermere?); set STACK_TELNET_PORT"
    note "$OS $(uname -m) -> $(target)"

    say "Settings ($CONF)"
    [ -n "${STACK_PASSWORD:-}" ] || save_setting STACK_PASSWORD "$(random_password)"
    for k in STACK_USER STACK_NAME STACK_RACE STACK_CLASS STACK_LEVEL STACK_TELNET_PORT STACK_ADMIN_PORT STACK_DAYDREAM_PORT; do
        if [ -n "${!k:-}" ]; then save_setting "$k" "${!k}"; fi
    done
    note "test login: $USER_NAME / $STACK_PASSWORD"

    say "silvermere"
    fetch_silvermere "${VERSION:-}"
    write_silvermere_config
    svc_active silvermere 2>/dev/null && down_silvermere
    # The account first: setup then has a sysop and asks nothing.
    sm migrate >/dev/null
    seed_account_only
    stock_world

    say "The test character"
    seed

    say "Daydream"
    fetch_daydream ""
    mkdir -p "$DD/home"

    say "Services"
    silvermere_service
    daydream_service
    up_silvermere
    up_daydream

    say "SyncTERM"
    if install_syncterm; then
        write_syncterm_entry
        connect || true
    fi

    install_self
    say "Ready"
    status
    echo
    echo "To check an edit in the game: daydream-stack edit, change it in Daydream, daydream-stack play."
}

has_account() { sm account list 2>/dev/null | grep -qiw -- "$USER_NAME"; }

# The account alone (before `setup`, which would otherwise ask for a sysop on the terminal).
seed_account_only() {
    if ! has_account; then
        printf '%s\n' "$STACK_PASSWORD" | sm account create "$USER_NAME" --sysop --password-stdin
    fi
}

install_self() {
    local script
    script=$(abspath "$0")
    mkdir -p "$(dirname "$SELF")"
    if [ "$script" != "$(abspath "$SELF" 2>/dev/null || echo)" ]; then
        install -m 755 "$script" "$SELF.new" && mv -f "$SELF.new" "$SELF"
        note "installed as $SELF"
    fi
}

update() {
    local c=${1:-all} v="" back=0
    shift || true
    while [ $# -gt 0 ]; do
        case "$1" in
        --version) v=$2; shift 2 ;;
        --version=*) v=${1#--version=}; shift ;;
        --rollback) back=1; shift ;;
        *) die "update [silvermere|daydream|syncterm] [--version V | --rollback]" ;;
        esac
    done
    case "$c" in
    all) update silvermere; update daydream; update syncterm ;;
    silvermere)
        say "silvermere"
        local before after
        before=$(version_of "$SM")
        if [ "$back" = 1 ]; then rollback "$SM"; else fetch_silvermere "$v"; fi
        after=$(version_of "$SM")
        if [ "$before" != "$after" ]; then
            local up=0
            svc_active silvermere && up=1
            [ "$up" = 1 ] && down_silvermere
            note "backing up the database first"
            sm backup | sed 's/^/    /'
            [ "$up" = 1 ] && up_silvermere
        fi
        note "silvermere $after"
        ;;
    daydream)
        say "Daydream"
        local before
        before=$(version_of "$DD")
        if [ "$back" = 1 ]; then rollback "$DD"; else fetch_daydream "$v"; fi
        if [ "$before" != "$(version_of "$DD")" ] && svc_active daydream; then
            svc_start daydream
            wait_port "$DD_PORT" 60 || die "Daydream did not start; $(logs_cmd daydream)"
        fi
        note "daydream $(version_of "$DD")"
        ;;
    syncterm)
        say "SyncTERM"
        if [ "$OS" != Darwin ] && command -v apt-get >/dev/null; then
            sudo apt-get install --only-upgrade -y syncterm || true
        else
            note "update SyncTERM from https://syncterm.bbsdev.net/"
        fi
        ;;
    *) die "update what? silvermere, daydream or syncterm" ;;
    esac
}

each() {
    local verb=$1 c=${2:-all}
    case "$c" in
    all)
        if [ "$verb" = up ]; then up_silvermere; up_daydream; else down_daydream; down_silvermere; fi ;;
    silvermere | daydream) "${verb}_$c" ;;
    *) die "$verb [silvermere|daydream]" ;;
    esac
}

cmd=${1:-help}
shift || true
case "$cmd" in
init) VERSION=""; init ;;
up | down) each "$cmd" "${1:-all}" ;;
connect) connect ;;
edit)
    say "Stopping silvermere so Daydream can write its database"
    down_silvermere
    echo "Now open the silvermere game in Daydream (http://127.0.0.1:$DD_PORT/), make the edit and write it back."
    echo "Then: daydream-stack play"
    ;;
play)
    say "Starting silvermere"
    up_silvermere
    connect
    ;;
seed) say "The test character"; seed ;;
update) update "$@" ;;
reset)
    say "Reset: a backup, then the stock world and a fresh character"
    down_silvermere
    sm backup | sed 's/^/    /'
    mv "$SM/data/silvermere.db" "$SM/data/silvermere.db.before-reset-$(date +%Y%m%d%H%M%S)"
    rm -f "$SM/data/silvermere.db-wal" "$SM/data/silvermere.db-shm"
    sm migrate >/dev/null
    seed_account_only
    stock_world
    seed
    up_silvermere
    ;;
status) status ;;
logs)
    case "${1:-}" in silvermere | daydream) ;; *) die "logs silvermere, or logs daydream" ;; esac
    eval "$(logs_cmd "$1")"
    ;;
remove)
    say "Removing the services (the data in $HOME_DIR stays)"
    down_silvermere || true
    svc_remove daydream
    svc_remove silvermere
    ;;
help | -h | --help) sed -n '2,32p' "$0" | sed 's/^# \{0,1\}//' ;;
*) die "unknown command $cmd (daydream-stack help)" ;;
esac
