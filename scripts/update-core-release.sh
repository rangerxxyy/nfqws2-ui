#!/bin/sh
# Verified nfqws2 Entware package update with the previous package as rollback.
set -eu
TAG=${1:-}; ASSET=${2:-}; HASH=${3:-}; OLD_TAG=${4:-}; OLD_ASSET=${5:-}; OLD_HASH=${6:-}; ID=${7:-}
valid_tag() { printf '%s\n' "$1" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'; }
valid_hash() { [ "${#1}" -eq 64 ] && case "$1" in *[!0-9a-fA-F]*) return 1 ;; *) return 0 ;; esac; }
if ! valid_tag "$TAG" || ! valid_tag "$OLD_TAG"; then echo 'invalid release tag' >&2; exit 2; fi
if ! valid_hash "$HASH" || ! valid_hash "$OLD_HASH"; then echo 'invalid checksum' >&2; exit 2; fi
case "$ID" in *[!0-9a-fA-F]*|'') echo 'invalid operation id' >&2; exit 2 ;; esac
[ "${#ID}" -eq 32 ] || { echo 'invalid operation id' >&2; exit 2; }
case "$ASSET" in nfqws2-keenetic_[0-9]*.[0-9]*.[0-9]*_aarch64-3.10.ipk) ;; *) echo 'invalid target package' >&2; exit 2 ;; esac
case "$OLD_ASSET" in nfqws2-keenetic_[0-9]*.[0-9]*.[0-9]*_aarch64-3.10.ipk) ;; *) echo 'invalid rollback package' >&2; exit 2 ;; esac
case "$ASSET$OLD_ASSET" in *[!A-Za-z0-9._-]*|*..*) echo 'invalid package filename' >&2; exit 2 ;; esac
DATA=/opt/var/lib/nfqws2-ui/update-operations
TMP="/tmp/nfqws2-core-update-$ID"
BASE="https://github.com/nfqws/nfqws2-keenetic/releases/download"
set_state() { state=$1; shift; printf '%s\n%s\n' "$state" "$*" > "$DATA/$ID.state.tmp"; mv -f "$DATA/$ID.state.tmp" "$DATA/$ID.state"; }
# This callback is invoked by the EXIT trap.
# shellcheck disable=SC2317
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT
[ "$(id -u)" = 0 ] || { set_state failed 'Must run as root.'; exit 1; }
if ! command -v wget >/dev/null 2>&1 || ! command -v sha256sum >/dev/null 2>&1 || [ ! -x /opt/bin/opkg ]; then
	set_state failed 'wget, sha256sum, or Entware opkg is missing.'
	exit 1
fi
mkdir -p "$TMP" "$DATA"; chmod 700 "$DATA"
set_state downloading "Downloading nfqws2 $TAG and rollback package $OLD_TAG."
wget -q -T 30 -O "$TMP/$ASSET" "$BASE/$TAG/$ASSET" || { set_state failed 'Could not download selected nfqws2 package.'; exit 1; }
wget -q -T 30 -O "$TMP/$OLD_ASSET" "$BASE/$OLD_TAG/$OLD_ASSET" || { set_state failed 'Could not download rollback package; update cancelled.'; exit 1; }
set_state verifying 'Verifying GitHub SHA-256 package digests.'
[ "$(sha256sum "$TMP/$ASSET" | awk '{print $1}')" = "$HASH" ] || { set_state failed 'Selected package SHA-256 mismatch.'; exit 1; }
[ "$(sha256sum "$TMP/$OLD_ASSET" | awk '{print $1}')" = "$OLD_HASH" ] || { set_state failed 'Rollback package SHA-256 mismatch.'; exit 1; }
set_state installing "Installing nfqws2 $TAG with Entware opkg."
if ! /opt/bin/opkg install "$TMP/$ASSET"; then
	set_state installing 'Package installation failed; restoring previous package.'
	if /opt/bin/opkg --force-downgrade install "$TMP/$OLD_ASSET"; then set_state rolled_back "Installation failed; restored nfqws2 $OLD_TAG."; else set_state failed 'Install failed and rollback failed; inspect opkg output.'; fi
	exit 1
fi
set_state health_check 'Restarting nfqws2 and checking service status.'
if /opt/etc/init.d/S51nfqws2 restart && /opt/etc/init.d/S51nfqws2 status; then
	set_state success "nfqws2 $TAG installed; service restart and status succeeded."
	exit 0
fi
set_state installing 'Service check failed; restoring previous package.'
if /opt/bin/opkg --force-downgrade install "$TMP/$OLD_ASSET" && /opt/etc/init.d/S51nfqws2 restart; then
	set_state rolled_back "Service check failed; restored nfqws2 $OLD_TAG."
else
	set_state failed 'Service check and package rollback failed; inspect nfqws2 service.'
fi
exit 1
