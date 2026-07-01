#!/usr/bin/env bash
set -u

make_cmd=$1
spec=$2
kernel_dir=$3
kunit_app=$4
kunit_handlers=$5
smoke_app=$6
test_plic_providers=${7:-}
default_overlay_map="tests/user/rootfs-overlay.map"
input_timeout=${USER_BOOT_INPUT_TIMEOUT:-30s}

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

status=0
summary_rows=""

record_row() {
    local name=$1
    local total=$2
    local pass=$3
    local fail=$4

    summary_rows="${summary_rows}$(printf '  %-18s total=%s pass=%s fail=%s' "$name" "$total" "$pass" "$fail")"$'\n'
}

add_summary() {
    local total=$1
    local pass=$2
    local fail=$3

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

run_with_delayed_input_log() {
    local log=$1
    local input=$2
    local ready_marker=$3
    shift 3
    local ch
    local buffer=""
    local sent=0
    local rc
    local proc_pid
    local input_fd
    local output_fd

    : > "$log"
    set +e
    coproc USER_BOOT_INPUT_PROC { timeout "$input_timeout" "$@" 2>&1; }
    proc_pid=$USER_BOOT_INPUT_PROC_PID
    output_fd=${USER_BOOT_INPUT_PROC[0]}
    input_fd=${USER_BOOT_INPUT_PROC[1]}
    while true; do
        if IFS= read -r -N 1 -t 1 ch <&"$output_fd"; then
            printf '%s' "$ch"
            printf '%s' "$ch" >> "$log"
            buffer="${buffer}${ch}"
            if [ "${#buffer}" -gt 512 ]; then
                buffer="${buffer: -512}"
            fi
            if [ "$sent" -eq 0 ] && [[ "$buffer" == *"$ready_marker"* ]]; then
                printf '%s' "$input" >&"$input_fd"
                sent=1
            fi
        elif ! kill -0 "$proc_pid" 2>/dev/null; then
            break
        fi
    done
    wait "$proc_pid"
    rc=$?
    exec {input_fd}>&- 2>/dev/null || true
    exec {output_fd}<&- 2>/dev/null || true
    if [ "$sent" -eq 0 ]; then
        printf 'user-boot input ready marker missing: %s\n' "$ready_marker"
        if [ "$rc" -eq 0 ]; then
            rc=1
        fi
    fi
    return "$rc"
}

run_command_case() {
    local name=$1
    local log=$2
    shift 2

    run_with_log "$log" "$@"
    local rc=$?
    local total=1
    local pass=0
    local fail=1
    if [ "$rc" -eq 0 ]; then
        pass=1
        fail=0
    else
        status=1
    fi
    record_row "$name" "$total" "$pass" "$fail"
    add_summary "$total" "$pass" "$fail"
}

run_user_boot_case() {
    local name=$1
    local log=$2
    shift 2

    run_with_log "$log" "$@"
    local rc=$?
    local exit_status
    local total=1
    local pass=0
    local fail=1
    exit_status=$(sed -n 's/.*user exit status=\([0-9][0-9]*\).*/\1/p' "$log" | tail -n 1)
    if [ "$rc" -eq 0 ] && [ "$exit_status" = "0" ]; then
        pass=1
        fail=0
    else
        status=1
        if [ -z "$exit_status" ]; then
            printf 'user-boot exit status missing in %s\n' "$name"
        else
            printf 'user-boot exit status for %s: %s\n' "$name" "$exit_status"
        fi
    fi
    record_row "$name" "$total" "$pass" "$fail"
    add_summary "$total" "$pass" "$fail"
}

run_user_boot_input_case() {
    local name=$1
    local log=$2
    local input=$3
    local expected_marker=$4
    local ready_marker=$5
    shift 5

    run_with_delayed_input_log "$log" "$input" "$ready_marker" "$@"
    local rc=$?
    local exit_status
    local total=1
    local pass=0
    local fail=1
    exit_status=$(sed -n 's/.*user exit status=\([0-9][0-9]*\).*/\1/p' "$log" | tail -n 1)
    if [ "$rc" -eq 0 ] && [ "$exit_status" = "0" ]; then
        if [ -z "$expected_marker" ] || grep -Fq "$expected_marker" "$log"; then
            pass=1
            fail=0
        else
            status=1
            printf 'user-boot expected marker missing for %s: %s\n' "$name" "$expected_marker"
        fi
    else
        status=1
        if [ -z "$exit_status" ]; then
            printf 'user-boot exit status missing in %s\n' "$name"
        else
            printf 'user-boot exit status for %s: %s\n' "$name" "$exit_status"
        fi
    fi
    record_row "$name" "$total" "$pass" "$fail"
    add_summary "$total" "$pass" "$fail"
}

run_user_boot_overlay_case() {
    local name=$1
    local provider=$2
    local overlay_map=$3
    local log=$4
    local image=$5

    run_user_boot_case "$name" "$log" "$make_cmd" run APP=user-boot PLIC_PROVIDER="$provider" \
        ROOTFS_OVERLAY_MAP="$overlay_map" VIRTIO_BLK_IMAGE="$image" FORCE=1 QEMU_APPEND="earlycon=sbi"
}

run_user_boot_overlay_append_case() {
    local name=$1
    local provider=$2
    local overlay_map=$3
    local append=$4
    local log=$5
    local image=$6

    run_user_boot_case "$name" "$log" "$make_cmd" run APP=user-boot PLIC_PROVIDER="$provider" \
        ROOTFS_OVERLAY_MAP="$overlay_map" VIRTIO_BLK_IMAGE="$image" FORCE=1 QEMU_APPEND="$append"
}

run_user_boot_no_overlay_append_case() {
    local name=$1
    local provider=$2
    local append=$3
    local log=$4
    local image=$5

    run_user_boot_case "$name" "$log" "$make_cmd" run APP=user-boot PLIC_PROVIDER="$provider" \
        ROOTFS_OVERLAY=none VIRTIO_BLK_IMAGE="$image" FORCE=1 QEMU_APPEND="$append"
}

run_user_boot_no_overlay_input_append_case() {
    local name=$1
    local provider=$2
    local append=$3
    local input=$4
    local expected_marker=$5
    local ready_marker=$6
    local log=$7
    local image=$8

    run_user_boot_input_case "$name" "$log" "$input" "$expected_marker" "$ready_marker" "$make_cmd" run APP=user-boot \
        PLIC_PROVIDER="$provider" ROOTFS_OVERLAY=none VIRTIO_BLK_IMAGE="$image" FORCE=1 QEMU_APPEND="$append"
}

run_kunit_case() {
    local name=$1
    local provider=$2
    local log=$3

    run_with_log "$log" "$make_cmd" -C "$kernel_dir" run APP="$kunit_app" PROBE_FILE="$kunit_handlers" PLIC_PROVIDER="$provider"
    local rc=$?
    local total
    local fail
    local cases
    local pass
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
    if [ "$pass" -lt 0 ]; then
        pass=0
    fi
    if [ "$rc" -ne 0 ] || [ "$fail" -ne 0 ]; then
        status=1
    fi
    record_row "$name" "$total" "$pass" "$fail"
    add_summary "$total" "$pass" "$fail"
}

run_smoke_case() {
    local name=$1
    local provider=$2
    local log=$3
    local image=$4

    run_with_log "$log" "$make_cmd" run APP="$smoke_app" PLIC_PROVIDER="$provider" \
        VIRTIO_BLK_IMAGE="$image" FORCE=1
    local rc=$?
    local counts
    local pass
    local fail
    local total
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
    if [ "$rc" -ne 0 ] || [ "$fail" -ne 0 ]; then
        status=1
    fi
    record_row "$name" "$total" "$pass" "$fail"
    add_summary "$total" "$pass" "$fail"
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
fi
if [ "$verify_rc" -ne 0 ]; then
    status=1
fi

summary_total=0
summary_pass=0
summary_fail=0

requested_init_overlay_map="$tmpdir/init-bin-ls-overlay.map"
cat > "$requested_init_overlay_map" <<'EOF'
# target      test        toolchain  link
/bin/ls       user_smoke  musl       dynamic
EOF
distro_sh_input=$'echo OK\nexit\n'

record_row "spec verify" "$verify_total" "$verify_pass" "$verify_fail"
add_summary "$verify_total" "$verify_pass" "$verify_fail"
run_command_case "run hello native" "$tmpdir/run-hello-native.log" "$make_cmd" run
run_user_boot_overlay_case "run user native" native "$default_overlay_map" \
    "$tmpdir/run-user-native.log" "$tmpdir/user-native-default.raw"
run_user_boot_overlay_append_case "run init=ls native" native "$requested_init_overlay_map" \
    "earlycon=sbi init=/bin/ls" "$tmpdir/run-init-ls-native.log" "$tmpdir/user-native-init-ls.raw"
run_user_boot_no_overlay_append_case "run distro ls native" native \
    "earlycon=sbi init=/bin/ls" "$tmpdir/run-distro-ls-native.log" "$tmpdir/user-native-distro-ls.raw"
run_user_boot_no_overlay_input_append_case "run distro sh native" native \
    "earlycon=sbi init=/bin/sh" "$distro_sh_input" "OK" "/ #" \
    "$tmpdir/run-distro-sh-native.log" "$tmpdir/user-native-distro-sh.raw"
run_kunit_case "KUnit native" native "$tmpdir/kunit-native.log"
run_smoke_case "app smoke native" native "$tmpdir/smoke-native.log" "$tmpdir/smoke-native.raw"

for provider in $test_plic_providers; do
    run_command_case "run hello $provider" "$tmpdir/run-hello-$provider.log" "$make_cmd" run PLIC_PROVIDER="$provider"
    run_user_boot_overlay_case "run user $provider" "$provider" "$default_overlay_map" \
        "$tmpdir/run-user-$provider.log" "$tmpdir/user-$provider-default.raw"
    run_user_boot_overlay_append_case "run init=ls $provider" "$provider" "$requested_init_overlay_map" \
        "earlycon=sbi init=/bin/ls" "$tmpdir/run-init-ls-$provider.log" "$tmpdir/user-$provider-init-ls.raw"
    run_user_boot_no_overlay_append_case "run distro ls $provider" "$provider" \
        "earlycon=sbi init=/bin/ls" "$tmpdir/run-distro-ls-$provider.log" "$tmpdir/user-$provider-distro-ls.raw"
    run_user_boot_no_overlay_input_append_case "run distro sh $provider" "$provider" \
        "earlycon=sbi init=/bin/sh" "$distro_sh_input" "OK" "/ #" \
        "$tmpdir/run-distro-sh-$provider.log" "$tmpdir/user-$provider-distro-sh.raw"
    run_kunit_case "KUnit $provider" "$provider" "$tmpdir/kunit-$provider.log"
    run_smoke_case "app smoke $provider" "$provider" "$tmpdir/smoke-$provider.log" "$tmpdir/smoke-$provider.raw"
done
printf '\nTest summary:\n'
printf '%s' "$summary_rows"
printf '  %-18s total=%s pass=%s fail=%s\n' "overall" "$summary_total" "$summary_pass" "$summary_fail"

exit "$status"
