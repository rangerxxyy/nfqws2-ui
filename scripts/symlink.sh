#!/bin/sh

# BusyBox mv follows a destination symlink to a directory. Remove the old
# pointer first so the new link replaces it instead of landing inside it.
replace_symlink() {
	link_path=$1
	destination=$2
	rm -f "$destination" || return 1
	mv -f "$link_path" "$destination"
}
