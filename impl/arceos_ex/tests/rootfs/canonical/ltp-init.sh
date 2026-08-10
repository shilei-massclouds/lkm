#!/bin/sh
# Publish a provider-neutral completion record after the stock LTP harness exits.
set -u

case ${1-} in
supported|frontier) selection=$1 ;;
*)
	printf 'usage: %s supported|frontier\n' "$0" >&2
	exit 2
	;;
esac

/opt/lkm/tests/ltp-select.sh "$selection"
status=$?
printf 'lkm-ltp: selection=%s status=%s\n' "$selection" "$status"
printf 'lkm-ltp: complete\n'
exit "$status"
