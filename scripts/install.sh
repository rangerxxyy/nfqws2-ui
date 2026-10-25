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
COMMITTED=0
SWITCHED=0

fail() { echo "nfqws2-ui installer: $*" >&2; exit 1; }

download_and_verify_release_archive() {
	DR_BASE=$1
	DR_ASSET=$2
	DR_SHA_ASSET=$3
	DR_DIRECTORY=$4
	DR_MAX_ATTEMPTS=3
	DR_ATTEMPT=1
	DR_DOWNLOAD_ERROR='could not verify release archive SHA256'
	while [ "$DR_ATTEMPT" -le "$DR_MAX_ATTEMPTS" ]; do
		DR_CACHE_BUSTER="$VERSION-$(date +%s)-$$-$DR_ATTEMPT"
		if ! "$WGET" -q -O "$DR_DIRECTORY/$DR_ASSET" "$DR_BASE/$DR_ASSET?nfqws2-ui-cache=$DR_CACHE_BUSTER"; then
			DR_DOWNLOAD_ERROR='cannot download release archive'
		elif ! "$WGET" -q -O "$DR_DIRECTORY/$DR_SHA_ASSET" "$DR_BASE/$DR_SHA_ASSET?nfqws2-ui-cache=$DR_CACHE_BUSTER"; then
			DR_DOWNLOAD_ERROR='cannot download SHA256 file'
		else
			DR_EXPECTED=$(awk 'NR==1 { print $1 }' "$DR_DIRECTORY/$DR_SHA_ASSET")
			case "$DR_EXPECTED" in
				''|*[!0-9a-fA-F]*) DR_DOWNLOAD_ERROR='invalid SHA256 manifest' ;;
				*)
					DR_ACTUAL=$(sha256sum "$DR_DIRECTORY/$DR_ASSET" | awk '{print $1}')
					if [ "$DR_EXPECTED" = "$DR_ACTUAL" ]; then
						return 0
					fi
					DR_DOWNLOAD_ERROR='release archive SHA256 mismatch'
					;;
			esac
		fi
		DR_ATTEMPT=$((DR_ATTEMPT + 1))
	done
	DR_DOWNLOAD_ERROR="$DR_DOWNLOAD_ERROR after $DR_MAX_ATTEMPTS cache-busted attempts"
	return 1
}

# cleanup is called by EXIT/HUP/INT/TERM traps.
# shellcheck disable=SC2317,SC2329
cleanup() {
	if [ "$COMMITTED" != 1 ] && [ -d "$STAGE" ]; then
		CURRENT=$(readlink "$ROOT/current" 2>/dev/null || true)
		[ "$CURRENT" = "$STAGE" ] || rm -rf "$STAGE"
	fi
	rm -rf "$TMP"
}
# rollback is called only after the install transaction has switched versions.
# shellcheck disable=SC2317
rollback() {
	[ "$SWITCHED" = 1 ] || return 0
	/opt/etc/init.d/S99nfqws2-ui stop >/dev/null 2>&1 || true
	if [ -n "$PREVIOUS" ]; then
		ln -s "$PREVIOUS" "$ROOT/current.rollback.$$"
		replace_symlink "$ROOT/current.rollback.$$" "$ROOT/current" || return 1
		/opt/etc/init.d/S99nfqws2-ui restart >/dev/null 2>&1 || true
	else
		/opt/etc/init.d/S99nfqws2-ui stop >/dev/null 2>&1 || true
		rm -f "$ROOT/current"
		rm -f /opt/etc/init.d/S99nfqws2-ui
	fi
	[ ! -d "$STAGE" ] || rm -rf "$STAGE"
	SWITCHED=0
}

# abort is invoked indirectly by the HUP/INT/TERM traps.
# shellcheck disable=SC2317,SC2329
abort() {
	if [ "$SWITCHED" = 1 ]; then rollback || true; fi
	cleanup
	exit 1
}
trap cleanup EXIT
trap abort HUP INT TERM

case "$VERSION" in ''|*@VERSION@*|*[!A-Za-z0-9._+-]*) fail 'invalid release version embedded in install.sh' ;; esac
[ "$(id -u)" = 0 ] || fail 'run as root'
[ -x /opt/bin/wget ] || command -v wget >/dev/null 2>&1 || fail 'wget is required'
command -v tar >/dev/null 2>&1 || fail 'tar is required'
if ! command -v sha256sum >/dev/null 2>&1; then
	fail 'sha256sum is required; install it first with: opkg update && opkg install coreutils-sha256sum'
fi
ARCH=$(uname -m)
case "$ARCH" in aarch64|arm64) ;; *) fail "unsupported architecture: $ARCH (release targets aarch64)" ;; esac

mkdir -p "$TMP" "$ROOT/releases" "$DATA" "$RUN" /opt/var/log
chmod 700 "$DATA" "$RUN"
WGET=$(command -v wget)
download_and_verify_release_archive "$BASE" "$ASSET" "$SHA_ASSET" "$TMP" || fail "$DR_DOWNLOAD_ERROR"

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
[ -r "$STAGE/scripts/symlink.sh" ] || { rm -rf "$STAGE"; fail 'archive has no symlink helper'; }
# The sourced path is resolved only after the verified archive is extracted.
# shellcheck disable=SC1091
. "$STAGE/scripts/symlink.sh"

# Remove the credential file created by older panel versions; authentication
# is no longer part of this version's UI or API.
rm -f "$ETC/password.hash"
rmdir "$ETC" 2>/dev/null || true

mkdir -p /opt/etc/init.d
cp "$STAGE/scripts/S99nfqws2-ui" /opt/etc/init.d/S99nfqws2-ui
chmod 755 /opt/etc/init.d/S99nfqws2-ui
if [ -L "$ROOT/current" ]; then PREVIOUS=$(readlink "$ROOT/current"); fi
if ! /opt/etc/init.d/S99nfqws2-ui stop; then fail 'cannot stop the current service; update cancelled'; fi
ln -s "$STAGE" "$ROOT/current.new.$$"
SWITCHED=1
if ! replace_symlink "$ROOT/current.new.$$" "$ROOT/current"; then rollback; fail 'cannot switch the current release; previous version restored'; fi
if ! /opt/etc/init.d/S99nfqws2-ui start; then rollback; fail 'service failed to start; previous version restored'; fi

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
				rm -rf "$old_release" 2>/dev/null || true
			done
			if [ -r "$STAGE/scripts/nfqws2-ui-ctl" ]; then
				mkdir -p /opt/bin
				cp "$STAGE/scripts/nfqws2-ui-ctl" /opt/bin/nfqws2-ui-ctl
				chmod 755 /opt/bin/nfqws2-ui-ctl
			fi
			COMMITTED=1
			echo "Панель nfqws2-ui $VERSION успешно установлена. Откройте её по адресу:"
			LAN_IPS=''
			if command -v ip >/dev/null 2>&1; then
				LAN_IPS=$(ip -o -4 addr show scope global 2>/dev/null | awk '$2 ~ /^br/ { split($4, a, "/"); print a[1] }')
				if [ -z "$LAN_IPS" ]; then
					LAN_IPS=$(ip -o -4 addr show scope global 2>/dev/null | awk '{ split($4, a, "/"); print a[1] }')
				fi
			fi
			if [ -z "$LAN_IPS" ] && command -v ifconfig >/dev/null 2>&1; then
				LAN_IPS=$(ifconfig 2>/dev/null | awk '{ for (i=1; i<=NF; i++) { if ($i == "inet" && $(i+1) != "127.0.0.1") print $(i+1); else if ($i ~ /^addr:/) { ip=substr($i, 6); if (ip != "127.0.0.1") print ip } } }')
			fi
			if [ -n "$LAN_IPS" ]; then
				for LAN_IP in $LAN_IPS; do
					URL="http://$LAN_IP:$PORT/"
					printf '\033]8;;%s\033\\%s\033]8;;\033\\\n' "$URL" "$URL"
				done
			else
				echo "Router IP could not be detected automatically. Use its LAN address: http://<router-lan-ip>:$PORT/"
			fi
			exit 0
		fi
	fi
	tries=$((tries + 1))
	sleep 1
done
rollback
fail 'HTTP health check failed; previous version restored'
