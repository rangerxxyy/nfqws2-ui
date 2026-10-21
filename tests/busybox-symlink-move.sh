#!/bin/sh
set -eu

BUSYBOX=$(command -v busybox) || { echo 'busybox is required for this regression test' >&2; exit 1; }
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
mkdir -p "$TMP/releases/old" "$TMP/releases/new"
ln -s "$TMP/releases/old" "$TMP/current"
ln -s "$TMP/releases/new" "$TMP/current.new"

"$BUSYBOX" mv -f "$TMP/current.new" "$TMP/current"

CURRENT=$(readlink "$TMP/current")
[ "$CURRENT" = "$TMP/releases/new" ] || {
	echo "BusyBox mv followed the destination directory symlink; current now points to $CURRENT" >&2
	exit 1
}
[ ! -e "$TMP/releases/old/current.new" ] || {
	echo 'BusyBox mv placed the new link inside the old release directory' >&2
	exit 1
}
echo 'BusyBox mv safely replaced the current symlink'
