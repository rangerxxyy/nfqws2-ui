#!/bin/sh
set -eu

BUSYBOX=$(command -v busybox) || { echo 'busybox is required for this regression test' >&2; exit 1; }
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")/../scripts" && pwd)
# shellcheck source=scripts/symlink.sh
. "$SCRIPT_DIR/symlink.sh"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
mkdir -p "$TMP/releases/old" "$TMP/releases/new"
ln -s "$TMP/releases/old" "$TMP/current"
ln -s "$TMP/releases/new" "$TMP/current.new"

mv() { "$BUSYBOX" mv "$@"; }
replace_symlink "$TMP/current.new" "$TMP/current"

CURRENT=$(readlink "$TMP/current")
[ "$CURRENT" = "$TMP/releases/new" ] || {
	echo "replacement helper failed to switch current; it points to $CURRENT" >&2
	exit 1
}
[ ! -e "$TMP/releases/old/current.new" ] || {
	echo 'replacement helper placed the new link inside the old release directory' >&2
	exit 1
}

ln -s "$TMP/releases/old" "$TMP/current.rollback"
replace_symlink "$TMP/current.rollback" "$TMP/current"
CURRENT=$(readlink "$TMP/current")
[ "$CURRENT" = "$TMP/releases/old" ] || {
	echo "replacement helper failed to restore previous release; current points to $CURRENT" >&2
	exit 1
}
echo 'BusyBox release switch and rollback passed'
