#!/usr/bin/env bash
set -u

make_cmd=$1
spec=$2
kernel_dir=$3
kunit_app=$4
kunit_handlers=$5
smoke_app=$6

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

status=0

print_row() {
    local name=$1
    local total=$2
    local pass=$3
    local fail=$4
    printf '  %-18s total=%s pass=%s fail=%s\n' "$name" "$total" "$pass" "$fail"
}

run_with_log() {
    local log=$1
    shift
    set +e
    "$@" 2>&1 | tee "$log"
    local rc=${PIPESTATUS[0]}
    return "$rc"
}

run_with_log "$tmpdir/verify.log" "$make_cmd" verify REPORT=text SPEC="$spec"
verify_rc=$?
if [ "$verify_rc" -eq 0 ]; then
    verify_total=1
    verify_pass=1
    verify_fail=0
else
    verify_total=1
    verify_pass=0
    verify_fail=1
    status=1
fi

run_with_log "$tmpdir/kunit.log" "$make_cmd" -C "$kernel_dir" run APP="$kunit_app" PROBE_FILE="$kunit_handlers"
kunit_rc=$?
kunit_total=$(sed -n 's/.*1\.\.\([0-9][0-9]*\).*/\1/p' "$tmpdir/kunit.log" | awk 'BEGIN { max = 0 } { if ($1 > max) max = $1 } END { print max }')
kunit_fail=$(sed -n 's/.*not ok [0-9][0-9]* .*/x/p' "$tmpdir/kunit.log" | wc -l)
kunit_cases=$(awk '/^  (not )?ok [0-9]+ / { count++ } END { print count + 0 }' "$tmpdir/kunit.log")
if [ "$kunit_rc" -ne 0 ] && [ "$kunit_total" -eq 0 ]; then
    kunit_total=1
    kunit_fail=1
fi
if [ "$kunit_rc" -eq 0 ] && [ "$kunit_total" -ne 0 ] && [ "$kunit_cases" -ne "$kunit_total" ]; then
    printf 'KUnit plan mismatch: plan=%s cases=%s\n' "$kunit_total" "$kunit_cases"
    kunit_fail=$((kunit_fail + 1))
fi
kunit_pass=$((kunit_total - kunit_fail))
if [ "$kunit_pass" -lt 0 ]; then
    kunit_pass=0
fi
if [ "$kunit_rc" -ne 0 ] || [ "$kunit_fail" -ne 0 ]; then
    status=1
fi

run_with_log "$tmpdir/smoke.log" "$make_cmd" run APP="$smoke_app"
smoke_rc=$?
smoke_counts=$(sed -n 's/.*passed=\([0-9][0-9]*\) failed=\([0-9][0-9]*\) total=\([0-9][0-9]*\).*/\1 \2 \3/p' "$tmpdir/smoke.log" | tail -n 1)
if [ -n "$smoke_counts" ]; then
    set -- $smoke_counts
    smoke_pass=$1
    smoke_fail=$2
    smoke_total=$3
else
    smoke_total=1
    smoke_pass=0
    smoke_fail=1
fi
if [ "$smoke_rc" -ne 0 ] || [ "$smoke_fail" -ne 0 ]; then
    status=1
fi

summary_total=$((verify_total + kunit_total + smoke_total))
summary_pass=$((verify_pass + kunit_pass + smoke_pass))
summary_fail=$((verify_fail + kunit_fail + smoke_fail))

printf '\nTest summary:\n'
print_row "spec verify" "$verify_total" "$verify_pass" "$verify_fail"
print_row "KUnit checkpoints" "$kunit_total" "$kunit_pass" "$kunit_fail"
print_row "app smoke" "$smoke_total" "$smoke_pass" "$smoke_fail"
printf '  %-18s total=%s pass=%s fail=%s\n' "overall" "$summary_total" "$summary_pass" "$summary_fail"

exit "$status"
