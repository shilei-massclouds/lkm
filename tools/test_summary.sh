#!/usr/bin/env bash
set -u

make_cmd=$1
spec=$2
kernel_dir=$3
kunit_app=$4
kunit_handlers=$5
smoke_app=$6
test_plic_providers=${7:-}

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

status=0
summary_total=0
summary_pass=0
summary_fail=0
summary_rows=""

record_row() {
    local name=$1
    local total=$2
    local pass=$3
    local fail=$4

    summary_rows="${summary_rows}$(printf '  %-24s total=%s pass=%s fail=%s' "$name" "$total" "$pass" "$fail")"$'\n'
    summary_total=$((summary_total + total))
    summary_pass=$((summary_pass + pass))
    summary_fail=$((summary_fail + fail))
}

run_with_log() {
    local log=$1
    shift
    set +e
    "$@" 2>&1 | tee "$log"
    local rc=${PIPESTATUS[0]}
    return "$rc"
}

run_command_case() {
    local name=$1
    local log=$2
    shift 2
    local pass=0
    local fail=1

    if run_with_log "$log" "$@"; then
        pass=1
        fail=0
    else
        status=1
    fi
    record_row "$name" 1 "$pass" "$fail"
}

run_kunit_case() {
    local name=$1
    local test_name=$2
    local log=$3
    local rc total fail cases pass

    run_with_log "$log" "$make_cmd" run TEST="$test_name"
    rc=$?
    total=$(sed -n 's/.*1\.\.\([0-9][0-9]*\).*/\1/p' "$log" | awk 'BEGIN { max = 0 } { if ($1 > max) max = $1 } END { print max }')
    fail=$(sed -n 's/.*not ok [0-9][0-9]* .*/x/p' "$log" | wc -l)
    cases=$(awk '/^  (not )?ok [0-9]+ / { count++ } END { print count + 0 }' "$log")
    if [ "$total" -eq 0 ]; then
        total=1
        fail=1
    elif [ "$rc" -eq 0 ] && [ "$cases" -ne "$total" ]; then
        printf 'KUnit plan mismatch: plan=%s cases=%s\n' "$total" "$cases"
        fail=$((fail + 1))
    fi
    pass=$((total - fail))
    if [ "$pass" -lt 0 ]; then pass=0; fi
    if [ "$rc" -ne 0 ] || [ "$fail" -ne 0 ]; then status=1; fi
    record_row "$name" "$total" "$pass" "$fail"
}

run_smoke_case() {
    local name=$1
    local test_name=$2
    local log=$3
    local rc counts pass fail total

    run_with_log "$log" "$make_cmd" run TEST="$test_name"
    rc=$?
    counts=$(sed -n 's/.*passed=\([0-9][0-9]*\) failed=\([0-9][0-9]*\) total=\([0-9][0-9]*\).*/\1 \2 \3/p' "$log" | tail -n 1)
    if [ -n "$counts" ]; then
        set -- $counts
        pass=$1
        fail=$2
        total=$3
    else
        total=1
        pass=0
        fail=1
    fi
    if [ "$rc" -ne 0 ] || [ "$fail" -ne 0 ]; then status=1; fi
    record_row "$name" "$total" "$pass" "$fail"
}

run_command_case "spec verify" "$tmpdir/verify.log" "$make_cmd" verify REPORT=text VERBOSE=1 SPEC="$spec"
run_command_case "delayed stdin" "$tmpdir/delayed-stdin.log" env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest tools.tests.test_delayed_stdin
run_command_case "basic runner" "$tmpdir/basic-runner.log" "$make_cmd" test-basic
run_command_case "composite runner" "$tmpdir/composite-runner.log" "$make_cmd" test-composite
run_command_case "checkpoints" "$tmpdir/checkpoints.log" "$make_cmd" test-checkpoints

# The only rootfs construction in the aggregate gate. Every following basic test
# verifies and reuses this template without invoking a builder.
run_command_case "rootfs canonical" "$tmpdir/rootfs-canonical.log" "$make_cmd" disk ROOTFS=canonical

for provider in native $test_plic_providers; do
    run_command_case "hello $provider" "$tmpdir/hello-$provider.log" "$make_cmd" run TEST="hello-$provider"
    run_command_case "user $provider" "$tmpdir/user-$provider.log" "$make_cmd" run TEST="user-smoke-$provider"
    run_command_case "requested $provider" "$tmpdir/requested-$provider.log" "$make_cmd" run TEST="requested-init-$provider"
    run_command_case "distro ls $provider" "$tmpdir/distro-ls-$provider.log" "$make_cmd" run TEST="distro-ls-$provider"
    run_command_case "distro sh $provider" "$tmpdir/distro-sh-$provider.log" "$make_cmd" run TEST="distro-sh-$provider"
    run_command_case "rc.local $provider" "$tmpdir/rc-local-$provider.log" "$make_cmd" run TEST="rc-local-$provider"
    run_command_case "BusyBox init login $provider" "$tmpdir/busybox-init-login-$provider.log" "$make_cmd" run TEST="busybox-init-login-$provider"
    run_kunit_case "KUnit $provider" "checkpoint-kunit-$provider" "$tmpdir/kunit-$provider.log"
    run_smoke_case "app smoke $provider" "kernel-smoke-$provider" "$tmpdir/smoke-$provider.log"
done

run_command_case "LTP uname native" "$tmpdir/ltp.log" "$make_cmd" run TEST="ltp"
run_command_case "LTP uname linux-object" "$tmpdir/ltp-lo.log" "$make_cmd" run TEST="ltp-lo"

printf '\nTest summary:\n'
printf '%s' "$summary_rows"
printf '  %-24s total=%s pass=%s fail=%s\n' "overall" "$summary_total" "$summary_pass" "$summary_fail"

if [ "$status" -ne 0 ]; then
    trap - EXIT
    printf 'Test logs retained in %s\n' "$tmpdir"
fi

exit "$status"
