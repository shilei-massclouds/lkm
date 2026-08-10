#!/bin/sh
# Convert one checked-in exact-name selection into stock run-syscalls.sh argv.
set -eu
set -f

case ${1-} in
supported|frontier) selection=$1 ;;
*)
	printf 'usage: %s supported|frontier\n' "$0" >&2
	exit 2
	;;
esac

selection_file=/opt/lkm/tests/ltp-$selection
set --
while IFS= read -r entry || [ -n "$entry" ]; do
	set -- "$@" "$entry"
done < "$selection_file"

exec /opt/ltp/run-syscalls.sh -- "$@"
