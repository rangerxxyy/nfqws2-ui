#!/bin/sh

# BusyBox ash / POSIX sh installer. Release packaging replaces @VERSION@.
set -eu
VERSION='@VERSION@'
OWNER='rangerxxyy'
REPO='nfqws2-ui'
BASE="https://github.com/$OWNER/$REPO/releases/download/v$VERSION"
ASSET="nfqws2-ui-v$VERSION-aarch64-unknown-linux-musl.tar.gz"
SHA_ASSET="$ASSET.sha256"
ROOT=/opt/share/nfqws2-ui
ETC=/opt/etc/nfqws2-ui
DATA=/opt/var/lib/nfqws2-ui
RUN=/opt/var/run/nfqws2-ui
STAGE="$ROOT/releases/$VERSION-$(date +%Y%m%d%H%M%S)-$$"
TMP="/tmp/nfqws2-ui-install-$$"
PREVIOUS=''
TTY_OFF=0

fail() { echo "nfqws2-ui installer: $*" >&2; exit 1; }
# cleanup is called by EXIT/HUP/INT/TERM traps.
# shellcheck disable=SC2317
cleanup() {
	if [ "$TTY_OFF" = 1 ]; then stty echo </dev/tty 2>/dev/null || true; printf '\n' >/dev/tty 2>/dev/null || true; fi
	rm -rf "$TMP"
}
# rollback is called only after the install transaction has switched versions.
# shellcheck disable=SC2317
rollback() {
	if [ -n "$PREVIOUS" ]; then
		ln -s "$PREVIOUS" "$ROOT/current.rollback.$$"
		mv -f "$ROOT/current.rollback.$$" "$ROOT/current"
		/opt/etc/init.d/S99nfqws2-ui restart >/dev/null 2>&1 || true
	else
		/opt/etc/init.d/S99nfqws2-ui stop >/dev/null 2>&1 || true
		rm -f "$ROOT/current"
		rm -f /opt/etc/init.d/S99nfqws2-ui
	fi
	[ ! -d "$STAGE" ] || rm -rf "$STAGE"
}
trap cleanup EXIT
trap 'cleanup; exit 1' HUP INT TERM

case "$VERSION" in ''|*@VERSION@*|*[!A-Za-z0-9._+-]*) fail 'invalid release version embedded in install.sh' ;; esac
[ "$(id -u)" = 0 ] || fail 'run as root'
[ -x /opt/bin/wget ] || command -v wget >/dev/null 2>&1 || fail 'wget is required'
command -v tar >/dev/null 2>&1 || fail 'tar is required'
if ! command -v sha256sum >/dev/null 2>&1; then
	if command -v opkg >/dev/null 2>&1; then
		if ! opkg update >/dev/null 2>&1 || ! opkg install coreutils-sha256sum >/dev/null 2>&1; then
			fail 'install coreutils-sha256sum with opkg first'
		fi
	else
		fail 'sha256sum is required (Entware package: coreutils-sha256sum)'
	fi
fi
ARCH=$(uname -m)
case "$ARCH" in aarch64|arm64) ;; *) fail "unsupported architecture: $ARCH (release targets aarch64)" ;; esac

mkdir -p "$TMP" "$ROOT/releases" "$ETC" "$DATA" "$RUN" /opt/var/log
chmod 700 "$ETC" "$DATA" "$RUN"
WGET=$(command -v wget)
"$WGET" -q -O "$TMP/$ASSET" "$BASE/$ASSET" || fail 'cannot download release archive'
"$WGET" -q -O "$TMP/$SHA_ASSET" "$BASE/$SHA_ASSET" || fail 'cannot download SHA256 file'
EXPECTED=$(awk 'NR==1 { print $1 }' "$TMP/$SHA_ASSET")
case "$EXPECTED" in ''|*[!0-9a-fA-F]*) fail 'invalid SHA256 manifest' ;; esac
ACTUAL=$(sha256sum "$TMP/$ASSET" | awk '{print $1}')
[ "$EXPECTED" = "$ACTUAL" ] || fail 'release archive SHA256 mismatch'

# Reject absolute/traversal tar names before extracting.
tar -tzf "$TMP/$ASSET" > "$TMP/entries" || fail 'release archive is invalid'
if tar -tvzf "$TMP/$ASSET" | awk 'substr($1, 1, 1) != "-" && substr($1, 1, 1) != "d" { bad=1 } END { exit bad ? 0 : 1 }'; then
	fail 'release archive contains links or special files'
fi
while IFS= read -r entry; do
	case "$entry" in /*|../*|*/../*|*/..) fail "unsafe archive path: $entry" ;; esac
done < "$TMP/entries"
mkdir -p "$STAGE"
tar -xzf "$TMP/$ASSET" -C "$STAGE" || { rm -rf "$STAGE"; fail 'cannot extract release'; }
[ -x "$STAGE/bin/nfqws2-ui" ] || { rm -rf "$STAGE"; fail 'archive has no executable'; }
[ -d "$STAGE/public" ] || { rm -rf "$STAGE"; fail 'archive has no web interface'; }

# Preserve an existing administrator password on updates. First install asks
# for a password on the controlling terminal; no secret is passed in argv.
if [ ! -s "$ETC/password.hash" ]; then
	[ -r /dev/tty ] || fail 'first install needs an interactive terminal to set the panel password'
	printf 'Create panel password (12+ characters): ' >/dev/tty
	stty -echo </dev/tty 2>/dev/null || true
	TTY_OFF=1
	IFS= read -r PASSWORD </dev/tty || fail 'could not read password'
	stty echo </dev/tty 2>/dev/null || true
	TTY_OFF=0
	printf '\nConfirm password: ' >/dev/tty
	stty -echo </dev/tty 2>/dev/null || true
	TTY_OFF=1
	IFS= read -r PASSWORD2 </dev/tty || fail 'could not read password confirmation'
	stty echo </dev/tty 2>/dev/null || true
	TTY_OFF=0
	printf '\n' >/dev/tty
	[ "$PASSWORD" = "$PASSWORD2" ] || fail 'passwords do not match'
	[ "${#PASSWORD}" -ge 12 ] || fail 'password must contain at least 12 characters'
	printf '%s' "$PASSWORD" | "$STAGE/bin/nfqws2-ui" hash-password > "$TMP/password.hash" || fail 'cannot hash password'
	unset PASSWORD PASSWORD2
	cp "$TMP/password.hash" "$ETC/password.hash"
	chmod 600 "$ETC/password.hash"
fi

mkdir -p /opt/etc/init.d
cp "$STAGE/scripts/S99nfqws2-ui" /opt/etc/init.d/S99nfqws2-ui
chmod 755 /opt/etc/init.d/S99nfqws2-ui
if [ -L "$ROOT/current" ]; then PREVIOUS=$(readlink "$ROOT/current"); fi
ln -s "$STAGE" "$ROOT/current.new.$$"
mv -f "$ROOT/current.new.$$" "$ROOT/current"
if ! /opt/etc/init.d/S99nfqws2-ui restart; then rollback; fail 'service failed to start; previous version restored'; fi

PORT=1002
tries=0
while [ "$tries" -lt 15 ]; do
	if [ -s "$RUN/pid" ] && [ -s /opt/var/run/nfqws2-ui.port ]; then
		PORT=$(cat /opt/var/run/nfqws2-ui.port)
		case "$PORT" in *[!0-9]*|'') PORT=1002 ;; esac
		if "$WGET" -q -O "$TMP/health" "http://127.0.0.1:$PORT/api/health" && grep -q '"ok":true' "$TMP/health"; then
			for old_release in "$ROOT"/releases/*; do
				[ -d "$old_release" ] || continue
				[ "$old_release" = "$STAGE" ] && continue
				[ -n "$PREVIOUS" ] && [ "$old_release" = "$PREVIOUS" ] && continue
				rm -rf "$old_release"
			done
			echo "nfqws2-ui $VERSION installed: http://<router-lan-ip>:$PORT/"
			exit 0
		fi
	fi
	tries=$((tries + 1))
	sleep 1
done
rollback
fail 'HTTP health check failed; previous version restored'
