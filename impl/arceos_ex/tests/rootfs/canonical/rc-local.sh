#!/bin/sh

echo "lkm-rc-local: begin"
/bin/ls /
status=$?
echo "lkm-rc-local: end status=$status"
if [ "$status" -eq 0 ]; then
    /sbin/poweroff -f
fi
exit "$status"
