from __future__ import annotations

import contextlib
import io
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
TOOL_PATH = REPO_ROOT / "tools" / "checkpoints" / "map_linux_checkpoints.py"

spec = importlib.util.spec_from_file_location("map_linux_checkpoints", TOOL_PATH)
assert spec is not None
map_linux_checkpoints = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules[spec.name] = map_linux_checkpoints
spec.loader.exec_module(map_linux_checkpoints)


MAIN_C = """
void setup_arch(char **cmdline) {}
void mm_core_init(void) {}
void sched_init(void) {}
void rest_init(void) {}
int kernel_execve(const char *init_filename, const char *const *argv,
                  const char *const *envp) { return 0; }
void async_synchronize_full(void) {}
void kprobe_free_init_mem(void) {}
void ftrace_free_init_mem(void) {}
void kgdb_free_init_mem(void) {}
void exit_boot_config(void) {}
void free_initmem(void) {}
void mark_readonly(void) {}
void pti_finalize(void) {}
void numa_default_policy(void) {}
void rcu_end_inkernel_boot(void) {}
void panic(const char *message) {}
void cpuset_init_smp(void) {}
void driver_init(void) {}
void init_irq_proc(void) {}
void do_ctors(void) {}
void do_initcalls(void) {}
void kunit_run_all_tests(void) {}
void wait_for_initramfs(void) {}
void console_on_rootfs(void) {}
int init_eaccess(const char *path) { return 0; }
void prepare_namespace(void) {}
void integrity_load_keys(void) {}
char *ramdisk_execute_command;

static int run_init_process(const char *init_filename)
{
    return lkm_checkpoint_record_payload_online(kernel_execve(init_filename, argv_init, envp_init));
}

static int try_to_run_init_process(const char *init_filename)
{
    int ret;

    ret = run_init_process(init_filename);

    if (ret && ret != -ENOENT) {
        pr_err("Starting init: %s exists but couldn't execute it (error %d)\\n",
               init_filename, ret);
    }

    return ret;
}

void start_kernel(void)
{
    char *command_line;

    setup_arch(&command_line);
    mm_core_init();
    sched_init();
    rest_init();
}

static noinline void __ref __noreturn rest_init(void)
{
    schedule_preempt_disabled();
}

static int __ref kernel_init(void *unused)
{
    int ret;

    async_synchronize_full();
    system_state = SYSTEM_FREEING_INITMEM;
    kprobe_free_init_mem();
    ftrace_free_init_mem();
    kgdb_free_init_mem();
    exit_boot_config();
    free_initmem();
    mark_readonly();
    pti_finalize();
    system_state = SYSTEM_RUNNING;
    numa_default_policy();
    rcu_end_inkernel_boot();

    do_sysctl_args();
    if (execute_command) {
        ret = run_init_process(execute_command);
        if (!ret)
            return 0;
        panic("Requested init failed.");
    }
    if (!try_to_run_init_process("/sbin/init") ||
        !try_to_run_init_process("/etc/init") ||
        !try_to_run_init_process("/bin/init") ||
        !try_to_run_init_process("/bin/sh"))
        return 0;
    panic("No working init found.");
}

static noinline void __init kernel_init_freeable(void)
{
    smp_prepare_cpus(setup_max_cpus);
    workqueue_init();
    init_mm_internals();
    rcu_init_tasks_generic();
    do_pre_smp_initcalls();
    lockup_detector_init();
    smp_init();
    sched_init_smp();
    workqueue_init_topology();
    async_init();
    padata_init();
    page_alloc_init_late();
    do_basic_setup();
    kunit_run_all_tests();
    wait_for_initramfs();
    console_on_rootfs();
    if (init_eaccess(ramdisk_execute_command) != 0) {
        ramdisk_execute_command = NULL;
        prepare_namespace();
    }
    integrity_load_keys();
}

static void __init do_basic_setup(void)
{
    cpuset_init_smp();
    driver_init();
    init_irq_proc();
    do_ctors();
    do_initcalls();
}
"""


MM_INIT_C = """
void __init mm_core_init(void)
{
    mem_init();
}
"""


SCHED_CORE_C = """
void __init sched_init(void)
{
    init_rt_bandwidth();
}
"""


DO_MOUNTS_C = """
void __init prepare_namespace(void)
{
    wait_for_device_probe();
    md_run_setup();
    if (initrd_load(saved_root_name))
        goto out;
    mount_root(saved_root_name);
out:
    devtmpfs_mount();
    init_mount(".", "/", NULL, MS_MOVE, NULL);
    init_chroot(".");
}
"""


HEAD_S = """
__HEAD
SYM_CODE_START(_start)
    j _start_kernel

    .global relocate_enable_mmu
relocate_enable_mmu:
    la a2, 1f
    csrw CSR_TVEC, a2
    csrw CSR_SATP, a0
1:
    load_global_pointer
    csrw CSR_SATP, a2
    ret

.Lsetup_trap_vector:
    la a0, handle_exception
    csrw CSR_TVEC, a0
    ret
SYM_CODE_END(_start)

SYM_CODE_START(_start_kernel)
    la a3, __bss_start
    la a4, __bss_stop
    ble a4, a3, .Lclear_bss_done
.Lclear_bss:
    REG_S zero, (a3)
    add a3, a3, RISCV_SZPTR
    blt a3, a4, .Lclear_bss
.Lclear_bss_done:
    mv a0, a1
    la a3, .Lsecondary_park
    csrw CSR_TVEC, a3
    call setup_vm
    call relocate_enable_mmu
    call .Lsetup_trap_vector
    tail start_kernel
SYM_CODE_END(_start_kernel)
"""


RISCV_MM_INIT_C = """
static void __init create_fdt_early_page_table(uintptr_t fix_fdt_va,
                                               uintptr_t dtb_pa)
{
    create_pmd_mapping(fixmap_pmd, fix_fdt_va, dtb_pa, PMD_SIZE, PAGE_KERNEL);
    dtb_early_va = (void *)fix_fdt_va + (dtb_pa & (PMD_SIZE - 1));
    dtb_early_pa = dtb_pa;
}

asmlinkage void __init setup_vm(uintptr_t dtb_pa)
{
    kernel_map.virt_addr = KERNEL_LINK_ADDR + kernel_map.virt_offset;
    kernel_map.phys_addr = (uintptr_t)(&_start);
    kernel_map.size = (uintptr_t)(&_end) - kernel_map.phys_addr;

    pt_ops_set_early();

    /* Setup early PGD for fixmap */
    create_pgd_mapping(early_pg_dir, FIXADDR_START,
                       fixmap_pgd_next, PGDIR_SIZE, PAGE_TABLE);
    create_pmd_mapping(fixmap_pmd, FIXADDR_START,
                       (uintptr_t)fixmap_pte, PMD_SIZE, PAGE_TABLE);

    /* Setup trampoline PGD and PMD */
    create_pgd_mapping(trampoline_pg_dir, kernel_map.virt_addr,
                       trampoline_pgd_next, PGDIR_SIZE, PAGE_TABLE);
    create_pmd_mapping(trampoline_pmd, kernel_map.virt_addr,
                       kernel_map.phys_addr, PMD_SIZE, PAGE_KERNEL_EXEC);

    create_kernel_page_table(early_pg_dir, true);
    create_fdt_early_page_table(__fix_to_virt(FIX_FDT), dtb_pa);
}
"""


READ_WRITE_C = """
ssize_t ksys_read(unsigned int fd, char __user *buf, size_t count)
{
    return vfs_read(fd, buf, count);
}

SYSCALL_DEFINE3(read, unsigned int, fd, char __user *, buf, size_t, count)
{
    return ksys_read(fd, buf, count);
}

SYSCALL_DEFINE3(write, unsigned int, fd, const char __user *, buf,
                size_t, count)
{
    return ksys_write(fd, buf, count);
}

SYSCALL_DEFINE3(writev, unsigned long, fd, const struct iovec __user *, vec,
                unsigned long, vlen)
{
    return do_writev(fd, vec, vlen, 0);
}
"""


OPEN_C = """
SYSCALL_DEFINE4(openat, int, dfd, const char __user *, filename, int, flags,
                umode_t, mode)
{
    if (force_o_largefile())
        flags |= O_LARGEFILE;
    return do_sys_open(dfd, filename, flags, mode);
}

SYSCALL_DEFINE1(close, unsigned int, fd)
{
    int retval;
    struct file *file;

    file = file_close_fd(fd);
    if (!file)
        return -EBADF;

    retval = filp_flush(file, current->files);
    return retval;
}
"""


STAT_C = """
SYSCALL_DEFINE4(newfstatat, int, dfd, const char __user *, filename,
                struct stat __user *, statbuf, int, flag)
{
    struct kstat stat;
    int error;

    error = vfs_fstatat(dfd, filename, &stat, flag);
    if (error)
        return error;
    return cp_new_stat(&stat, statbuf);
}
"""


EXEC_C = """
static int exec_mmap(struct mm_struct *mm)
{
    struct mm_struct *active_mm = current->active_mm;

    current->mm = mm;
    activate_mm(active_mm, mm);
    return 0;
}

int begin_new_exec(struct linux_binprm * bprm)
{
    int retval;

    retval = exec_mmap(bprm->mm);
    if (retval)
        return retval;
    bprm->mm = NULL;
    return 0;
}

static int do_execveat_common(int fd, struct filename *filename,
                              struct user_arg_ptr argv,
                              struct user_arg_ptr envp,
                              int flags)
{
    struct linux_binprm *bprm = alloc_bprm(fd, filename, flags);
    int retval;

    retval = copy_string_kernel(bprm->filename, bprm);
    if (retval < 0)
        return retval;
    bprm->exec = bprm->p;

    retval = copy_strings(bprm->envc, envp, bprm);
    if (retval < 0)
        return retval;

    retval = copy_strings(bprm->argc, argv, bprm);
    if (retval < 0)
        return retval;

    retval = bprm_execve(bprm);
    return retval;
}
"""


BIN_ELF_C = """
static int load_elf_binary(struct linux_binprm *bprm)
{
    struct elfhdr *elf_ex = (struct elfhdr *)bprm->buf;
    struct elfhdr *interp_elf_ex = NULL;
    struct elf_phdr *elf_phdata;
    struct elf_phdr *interp_elf_phdata = NULL;
    struct mm_struct *mm;
    int retval;

    elf_phdata = load_elf_phdrs(elf_ex, bprm->file);
    if (!elf_phdata)
        return -ENOEXEC;

    interp_elf_phdata = load_elf_phdrs(interp_elf_ex, interpreter);
    if (!interp_elf_phdata)
        return -ELIBBAD;

    retval = begin_new_exec(bprm);
    if (retval)
        return retval;

    retval = setup_arg_pages(bprm, randomize_stack_top(STACK_TOP),
                             executable_stack);
    if (retval < 0)
        return retval;

    error = elf_load(bprm->file, load_bias + vaddr, elf_ppnt,
                     elf_prot, elf_flags, total_size);

    retval = create_elf_tables(bprm, elf_ex, interp_load_addr,
                               e_entry, phdr_addr);
    if (retval < 0)
        return retval;

    mm = current->mm;
    mm->start_stack = bprm->p;

    START_THREAD(elf_ex, regs, elf_entry, bprm->p);
    return 0;
}
"""


PROCESS_C = """
void start_thread(struct pt_regs *regs, unsigned long pc,
                  unsigned long sp)
{
    regs->status = SR_PIE;
    regs->epc = pc;
    regs->sp = sp;
}
"""


ENTRY_S = """
SYM_CODE_START_NOALIGN(ret_from_exception)
    REG_L a0, PT_STATUS(sp)
    csrw CSR_STATUS, a0
    csrw CSR_EPC, a2
    REG_L x2, PT_SP(sp)
    sret
SYM_CODE_END(ret_from_exception)
"""


FORK_C = """
SYSCALL_DEFINE1(set_tid_address, int __user *, tidptr)
{
    current->clear_child_tid = tidptr;
    return task_pid_vnr(current);
}

pid_t kernel_clone(struct kernel_clone_args *args)
{
    struct task_struct *p;
    int trace = 0;

    p = copy_process(NULL, trace, NUMA_NO_NODE, args);
    if (IS_ERR(p))
        return PTR_ERR(p);
    wake_up_new_task(p);
    return pid_vnr(get_task_pid(p, PIDTYPE_PID));
}

#ifdef __ARCH_WANT_SYS_CLONE
#ifdef CONFIG_CLONE_BACKWARDS
SYSCALL_DEFINE5(clone, unsigned long, clone_flags, unsigned long, newsp,
                int __user *, parent_tidptr,
                unsigned long, tls,
                int __user *, child_tidptr)
#elif defined(CONFIG_CLONE_BACKWARDS2)
SYSCALL_DEFINE5(clone, unsigned long, newsp, unsigned long, clone_flags,
                int __user *, parent_tidptr,
                int __user *, child_tidptr,
                unsigned long, tls)
#else
SYSCALL_DEFINE5(clone, unsigned long, clone_flags, unsigned long, newsp,
                int __user *, parent_tidptr,
                int __user *, child_tidptr,
                unsigned long, tls)
#endif
{
    struct kernel_clone_args args = {
        .stack = newsp,
        .tls = tls,
    };
    return kernel_clone(&args);
}
#endif
"""


EXIT_C = """
void __noreturn do_group_exit(int exit_code)
{
    do_exit(exit_code);
}

long kernel_wait4(pid_t upid, int __user *stat_addr, int options,
                  struct rusage *ru)
{
    struct wait_opts wo;
    long ret;

    wo.wo_flags = options | WEXITED;
    ret = do_wait(&wo);
    if (ret > 0 && stat_addr && put_user(wo.wo_stat, stat_addr))
        ret = -EFAULT;
    return ret;
}

SYSCALL_DEFINE4(wait4, pid_t, upid, int __user *, stat_addr,
                int, options, struct rusage __user *, ru)
{
    struct rusage r;
    long err = kernel_wait4(upid, stat_addr, options, ru ? &r : NULL);

    return err;
}
"""


SIGNAL_C = """
SYSCALL_DEFINE4(rt_sigtimedwait, const sigset_t __user *, uthese,
                siginfo_t __user *, uinfo,
                const struct __kernel_timespec __user *, uts,
                size_t, sigsetsize)
{
    sigset_t these;

    if (sigsetsize != sizeof(sigset_t))
        return -EINVAL;
    if (copy_from_user(&these, uthese, sizeof(sigset_t)))
        return -EFAULT;

    return do_sigtimedwait(&these, uinfo, uts);
}
"""


class MapLinuxCheckpointsTests(unittest.TestCase):
    def _write_linux_fixture(self, tmp: str) -> Path:
        root = Path(tmp) / "linux"
        (root / "init").mkdir(parents=True)
        (root / "mm").mkdir(parents=True)
        (root / "kernel" / "sched").mkdir(parents=True)
        (root / "kernel").mkdir(parents=True, exist_ok=True)
        (root / "fs").mkdir(parents=True)
        (root / "arch" / "riscv" / "kernel").mkdir(parents=True)
        (root / "arch" / "riscv" / "mm").mkdir(parents=True)
        (root / "init" / "main.c").write_text(MAIN_C, encoding="utf-8")
        (root / "mm" / "mm_init.c").write_text(MM_INIT_C, encoding="utf-8")
        (root / "kernel" / "sched" / "core.c").write_text(SCHED_CORE_C, encoding="utf-8")
        (root / "kernel" / "fork.c").write_text(FORK_C, encoding="utf-8")
        (root / "kernel" / "exit.c").write_text(EXIT_C, encoding="utf-8")
        (root / "kernel" / "signal.c").write_text(SIGNAL_C, encoding="utf-8")
        (root / "fs" / "read_write.c").write_text(READ_WRITE_C, encoding="utf-8")
        (root / "fs" / "open.c").write_text(OPEN_C, encoding="utf-8")
        (root / "fs" / "stat.c").write_text(STAT_C, encoding="utf-8")
        (root / "fs" / "exec.c").write_text(EXEC_C, encoding="utf-8")
        (root / "fs" / "binfmt_elf.c").write_text(BIN_ELF_C, encoding="utf-8")
        (root / "init" / "do_mounts.c").write_text(DO_MOUNTS_C, encoding="utf-8")
        (root / "arch" / "riscv" / "kernel" / "head.S").write_text(HEAD_S, encoding="utf-8")
        (root / "arch" / "riscv" / "kernel" / "entry.S").write_text(
            ENTRY_S,
            encoding="utf-8",
        )
        (root / "arch" / "riscv" / "kernel" / "process.c").write_text(
            PROCESS_C,
            encoding="utf-8",
        )
        (root / "arch" / "riscv" / "mm" / "init.c").write_text(
            RISCV_MM_INIT_C,
            encoding="utf-8",
        )
        return root

    def test_fixture_maps_exact_rules_and_preserves_order(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=0,
                variant="KernelStarted",
                name="Kernel.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=108,
                variant="MmCoreInitPhaseReady",
                name="MmCoreInitPhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=352,
                variant="RootfsPhaseReady",
                name="RootfsPhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=401,
                variant="KernelInitFlowPayloadHandoffCommitted",
                name="KernelInitFlow.PayloadHandoffCommitted",
            ),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        self.assertEqual([record.checkpoint_index for record in mapped], [8, 0, 108, 352, 401])
        self.assertEqual(mapped[0].mapping_kind, "exact")
        self.assertEqual(mapped[0].linux_file, "arch/riscv/kernel/head.S")
        self.assertEqual(mapped[0].linux_symbol, "_start")
        self.assertEqual(mapped[1].linux_file, "init/main.c")
        self.assertEqual(mapped[1].linux_symbol, "start_kernel")
        self.assertEqual(mapped[1].mapping_kind, "exact")
        self.assertEqual(mapped[2].linux_file, "mm/mm_init.c")
        self.assertEqual(mapped[3].linux_symbol, "prepare_namespace")
        self.assertEqual(mapped[4].linux_symbol, "run_init_process")
        self.assertIn("definition", mapped[4].linux_anchor)

    def test_interrupt_leaf_lifecycle_additions_default_to_unmapped(self) -> None:
        names = [
            "IrqTimeInitPhase.Prepared",
            "LocalIrqEnablePhase.Prepared",
            "LocalIrqEnablePhase.Online",
            "IrqOpenPreparePhase.Prepared",
            "IrqOpenPreparePhase.Online",
            "ProcessPreparePhase.Prepared",
            "ProcessPreparePhase.Online",
        ]
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=560 + index,
                variant=name.replace(".", ""),
                name=name,
            )
            for index, name in enumerate(names)
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        self.assertEqual([record.checkpoint_name for record in mapped], names)
        self.assertTrue(all(record.mapping_kind == "unmapped" for record in mapped))

        rules = map_linux_checkpoints.default_mapping_rules()
        self.assertEqual(rules["IrqTimeInitPhase.Ready"].mapping_kind, "range")
        self.assertEqual(rules["LocalIrqEnablePhase.Ready"].mapping_kind, "exact")
        self.assertEqual(rules["ProcessPreparePhase.Ready"].mapping_kind, "range")

    def test_ap_phase_lifecycle_additions_default_to_unmapped(self) -> None:
        names = [
            "ApEntryPreludePhase.Prepared",
            "ApEntryPreludePhase.Online",
            "ApSmpCallinPhase.Prepared",
            "ApSmpCallinPhase.Online",
            "ApOnlineIdlePhase.Prepared",
            "ApOnlineIdlePhase.Online",
        ]
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=475 + index,
                variant=name.replace(".", ""),
                name=name,
            )
            for index, name in enumerate(names)
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        self.assertEqual([record.checkpoint_name for record in mapped], names)
        self.assertTrue(all(record.mapping_kind == "unmapped" for record in mapped))

    def test_mapping_ignores_generated_marker_comment_lines(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=0,
                variant="KernelStarted",
                name="Kernel.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=80,
                variant="CorePreparePhaseStarted",
                name="CorePreparePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=400,
                variant="PayloadPreparePhaseOnline",
                name="PayloadPreparePhase.Online",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            baseline = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)

            main_path = linux_tree / "init" / "main.c"
            main_text = main_path.read_text(encoding="utf-8")
            main_text = main_text.replace(
                "void start_kernel(void)\n",
                "/* LKM_CHECKPOINT name=Kernel.Started variant=KernelStarted fingerprint=sha256:demo */\n"
                "void start_kernel(void)\n",
            )
            main_text = main_text.replace(
                "    setup_arch(&command_line);\n",
                "    /* LKM_CHECKPOINT name=CorePreparePhase.Started variant=CorePreparePhaseStarted fingerprint=sha256:demo */\n"
                "    setup_arch(&command_line);\n",
            )
            main_text = main_text.replace(
                "    do_sysctl_args();\n",
                "    /* LKM_CHECKPOINT name=PayloadPreparePhase.Online variant=PayloadPreparePhaseOnline fingerprint=sha256:demo */\n"
                "    do_sysctl_args();\n",
            )
            main_path.write_text(main_text, encoding="utf-8")

            head_path = linux_tree / "arch" / "riscv" / "kernel" / "head.S"
            head_text = head_path.read_text(encoding="utf-8").replace(
                "SYM_CODE_START(_start)\n",
                "/* LKM_CHECKPOINT name=EntryPreludePhase.Started variant=EntryPreludePhaseStarted fingerprint=sha256:demo */\n"
                "SYM_CODE_START(_start)\n",
            )
            head_path.write_text(head_text, encoding="utf-8")

            annotated = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)

        self.assertEqual(baseline, annotated)

    def test_mapping_ignores_runtime_instrumentation_lines(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=0,
                variant="KernelStarted",
                name="Kernel.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=9,
                variant="EntryPreludePhaseReady",
                name="EntryPreludePhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=31,
                variant="TrampolineVmOnline",
                name="TrampolineVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=33,
                variant="EarlyVmOnline",
                name="EarlyVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=42,
                variant="KernelImageOnline",
                name="KernelImage.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=43,
                variant="EventStreamReady",
                name="EventStream.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=80,
                variant="ExceptionStreamReady",
                name="ExceptionStream.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=28,
                variant="EventStreamPrepared",
                name="EventStream.Prepared",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=29,
                variant="ExceptionStreamPrepared",
                name="ExceptionStream.Prepared",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=80,
                variant="CorePreparePhaseStarted",
                name="CorePreparePhase.Started",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            baseline = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)

            main_path = linux_tree / "init" / "main.c"
            main_text = main_path.read_text(encoding="utf-8")
            main_text = main_text.replace(
                "void setup_arch(char **cmdline) {}\n",
                "#include <linux/lkm_checkpoints.h>\n"
                "void setup_arch(char **cmdline) {}\n",
            )
            main_text = main_text.replace(
                "void start_kernel(void)\n{\n",
                "void start_kernel(void)\n{\n"
                "    lkm_checkpoint_record(LKM_CHECKPOINT_KERNEL_STARTED);\n",
            )
            main_text = main_text.replace(
                "    setup_arch(&command_line);\n",
                "    lkm_checkpoint_record(LKM_CHECKPOINT_CORE_PREPARE_PHASE_STARTED);\n"
                "    setup_arch(&command_line);\n",
            )
            main_path.write_text(main_text, encoding="utf-8")

            head_path = linux_tree / "arch" / "riscv" / "kernel" / "head.S"
            head_text = head_path.read_text(encoding="utf-8")
            head_text = head_text.replace(
                "__HEAD\n",
                "#include <linux/lkm_checkpoints.h>\n"
                "#ifdef CONFIG_LKM_CHECKPOINTS\n"
                "#ifndef CONFIG_RISCV_M_MODE\n"
                ".macro LKM_RUNTIME_CHECKPOINT checkpoint_id\n"
                "    nop\n"
                ".endm\n"
                "#endif\n"
                "#else\n"
                ".macro LKM_RUNTIME_CHECKPOINT checkpoint_id\n"
                ".endm\n"
                "#endif\n\n"
                "__HEAD\n",
            )
            head_text = head_text.replace(
                "    csrw CSR_SATP, a0\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_TRAMPOLINE_VM_ONLINE\n"
                "    csrw CSR_SATP, a0\n",
            )
            head_text = head_text.replace(
                "    load_global_pointer\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_KERNEL_IMAGE_ONLINE\n"
                "    load_global_pointer\n",
            )
            head_text = head_text.replace(
                "    csrw CSR_SATP, a2\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_EARLY_VM_ONLINE\n"
                "    csrw CSR_SATP, a2\n",
            )
            head_text = head_text.replace(
                "    la a0, handle_exception\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_EVENT_STREAM_READY\n"
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_EXCEPTION_STREAM_READY\n"
                "    la a0, handle_exception\n",
            )
            head_text = head_text.replace(
                "SYM_CODE_START(_start_kernel)\n",
                "SYM_CODE_START(_start_kernel)\n"
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_ENTRY_PRELUDE_PHASE_STARTED\n",
            )
            head_text = head_text.replace(
                "    csrw CSR_TVEC, a3\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_EVENT_STREAM_PREPARED\n"
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_EXCEPTION_STREAM_PREPARED\n"
                "    csrw CSR_TVEC, a3\n",
            )
            head_text = head_text.replace(
                "    tail start_kernel\n",
                "    LKM_RUNTIME_CHECKPOINT LKM_CHECKPOINT_ENTRY_PRELUDE_PHASE_READY\n"
                "    tail start_kernel\n",
            )
            head_text = head_text.replace(
                "SYM_CODE_END(_start_kernel)\n",
                "SYM_CODE_END(_start_kernel)\n\n"
                "#ifdef CONFIG_LKM_CHECKPOINTS\n"
                ".pushsection .data, \"aw\"\n"
                "lkm_checkpoint_count:\n"
                "    RISCV_PTR 0\n"
                ".popsection\n"
                "#endif\n",
            )
            head_path.write_text(head_text, encoding="utf-8")

            instrumented = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)

        self.assertEqual(baseline, instrumented)

    def test_range_rule_requires_ordered_anchors(self) -> None:
        record = map_linux_checkpoints.CheckpointInventoryRecord(
            index=10,
            variant="DemoRange",
            name="Demo.Range",
        )
        ordered = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsetup_arch\s*\(",
            end_anchor_pattern=r"\bsched_init\s*\(",
        )
        reversed_rule = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsched_init\s*\(",
            end_anchor_pattern=r"\bsetup_arch\s*\(",
        )

        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            mapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=linux_tree,
                rules={"Demo.Range": ordered},
            )[0]
            unmapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=linux_tree,
                rules={"Demo.Range": reversed_rule},
            )[0]

        self.assertEqual(mapped.mapping_kind, "range")
        self.assertIn("setup_arch", mapped.linux_anchor)
        self.assertIn("sched_init", mapped.linux_anchor)
        self.assertEqual(unmapped.mapping_kind, "unmapped")
        self.assertIn("out of order", unmapped.notes)

    def test_range_rule_with_missing_anchor_is_unmapped(self) -> None:
        record = map_linux_checkpoints.CheckpointInventoryRecord(
            index=11,
            variant="DemoMissingRange",
            name="Demo.MissingRange",
        )
        missing = map_linux_checkpoints.MappingRule(
            mapping_kind="range",
            linux_file="init/main.c",
            linux_symbol="start_kernel",
            confidence="medium",
            notes="fixture range",
            start_anchor_pattern=r"\bsetup_arch\s*\(",
            end_anchor_pattern=r"\bdoes_not_exist\s*\(",
        )

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=self._write_linux_fixture(tmp),
                rules={"Demo.MissingRange": missing},
            )[0]

        self.assertEqual(mapped.mapping_kind, "unmapped")
        self.assertIn("were not both found", mapped.notes)

    def test_assembly_fixture_maps_symbols_labels_and_instruction_anchors(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=8,
                variant="EntryPreludePhaseStarted",
                name="EntryPreludePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=9,
                variant="EntryPreludePhaseReady",
                name="EntryPreludePhase.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=31,
                variant="TrampolineVmOnline",
                name="TrampolineVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=37,
                variant="EarlyVmReady",
                name="EarlyVm.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=38,
                variant="EarlyVmOnline",
                name="EarlyVm.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=43,
                variant="EventStreamReady",
                name="EventStream.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_symbol, "_start")
        self.assertIn("definition line", by_name["EntryPreludePhase.Started"].linux_anchor)
        self.assertIn("tail start_kernel", by_name["EntryPreludePhase.Ready"].linux_anchor)
        self.assertEqual(by_name["TrampolineVm.Online"].linux_symbol, "relocate_enable_mmu")
        self.assertIn("csrw CSR_SATP, a0", by_name["TrampolineVm.Online"].linux_anchor)
        self.assertIn("create_kernel_page_table", by_name["EarlyVm.Ready"].linux_anchor)
        self.assertIn("csrw CSR_SATP, a2", by_name["EarlyVm.Online"].linux_anchor)
        self.assertIn("handle_exception", by_name["EventStream.Ready"].linux_anchor)

    def test_syscall_macro_parser_handles_wrappers_and_rejects_conditional_clone(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            linux_tree = self._write_linux_fixture(tmp)
            index = map_linux_checkpoints.LinuxSourceIndex(linux_tree)

            read = index.find_symbol("fs/read_write.c", "SYSCALL_DEFINE3(read)")
            write = index.find_symbol("fs/read_write.c", "SYSCALL_DEFINE3(write)")
            clone = index.find_symbol("kernel/fork.c", "SYSCALL_DEFINE5(clone)")

        self.assertIsNotNone(read)
        assert read is not None
        self.assertEqual(read.kind, "syscall_macro")
        self.assertIn("ksys_read", read.body)

        self.assertIsNotNone(write)
        assert write is not None
        self.assertEqual(write.kind, "syscall_macro")
        self.assertIn("ksys_write", write.body)

        self.assertIsNone(clone)

    def test_syscall_macro_missing_anchor_is_unmapped(self) -> None:
        record = map_linux_checkpoints.CheckpointInventoryRecord(
            index=12,
            variant="DemoMissingSyscallAnchor",
            name="Demo.MissingSyscallAnchor",
        )
        missing = map_linux_checkpoints.MappingRule(
            mapping_kind="exact",
            linux_file="fs/read_write.c",
            linux_symbol="SYSCALL_DEFINE3(read)",
            confidence="high",
            notes="fixture syscall macro",
            anchor_pattern=r"\bdoes_not_exist\s*\(",
        )

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                [record],
                linux_tree=self._write_linux_fixture(tmp),
                rules={"Demo.MissingSyscallAnchor": missing},
            )[0]

        self.assertEqual(mapped.mapping_kind, "unmapped")
        self.assertIn("Linux anchor", mapped.notes)

    def test_finalize_tail_rules_map_to_kernel_init_anchors(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=363,
                variant="AsyncFullSyncDeferredReady",
                name="AsyncFullSync.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=364,
                variant="SystemStateFreeingInitmemCheckpoint",
                name="SystemState.FreeingInitmemCheckpoint",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=365,
                variant="InitMemoryCleanupDeferredReady",
                name="InitMemoryCleanup.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=366,
                variant="KernelMappingProtectionDeferredReady",
                name="KernelMappingProtection.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=367,
                variant="PtiFinalizeTrimmedReady",
                name="PtiFinalize.TrimmedReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=368,
                variant="SystemStateOnline",
                name="SystemState.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=369,
                variant="RcuInkernelBootEnded",
                name="RcuCore.InkernelBootEnded",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=370,
                variant="RcuBootEndReady",
                name="RcuBootEnd.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=371,
                variant="SysctlArgsDeferredReady",
                name="SysctlArgs.DeferredReady",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        async_full_sync = by_name["AsyncFullSync.DeferredReady"]
        self.assertEqual(async_full_sync.linux_file, "init/main.c")
        self.assertEqual(async_full_sync.linux_symbol, "kernel_init")
        self.assertEqual(async_full_sync.mapping_kind, "exact")
        self.assertEqual(async_full_sync.confidence, "medium")
        self.assertIn("async_synchronize_full", async_full_sync.linux_anchor)
        self.assertIn("deferred finalize boundary", async_full_sync.notes)

        freeing = by_name["SystemState.FreeingInitmemCheckpoint"]
        self.assertEqual(freeing.mapping_kind, "exact")
        self.assertEqual(freeing.confidence, "high")
        self.assertIn("SYSTEM_FREEING_INITMEM", freeing.linux_anchor)

        cleanup = by_name["InitMemoryCleanup.DeferredReady"]
        self.assertEqual(cleanup.mapping_kind, "exact")
        self.assertEqual(cleanup.confidence, "medium")
        self.assertIn("free_initmem", cleanup.linux_anchor)
        self.assertIn("deferred cleanup boundary", cleanup.notes)

        mapping_protection = by_name["KernelMappingProtection.DeferredReady"]
        self.assertEqual(mapping_protection.linux_file, "init/main.c")
        self.assertEqual(mapping_protection.linux_symbol, "kernel_init")
        self.assertEqual(mapping_protection.mapping_kind, "exact")
        self.assertEqual(mapping_protection.confidence, "medium")
        self.assertIn("mark_readonly", mapping_protection.linux_anchor)
        self.assertIn("mapping-protection path", mapping_protection.notes)
        self.assertIn("deferred finalize boundary", mapping_protection.notes)

        pti = by_name["PtiFinalize.TrimmedReady"]
        self.assertEqual(pti.mapping_kind, "exact")
        self.assertEqual(pti.confidence, "medium")
        self.assertIn("pti_finalize", pti.linux_anchor)
        self.assertIn("trimmed/no-op fact", pti.notes)

        online = by_name["SystemState.Online"]
        self.assertEqual(online.linux_symbol, "kernel_init")
        self.assertEqual(online.confidence, "high")
        self.assertIn("SYSTEM_RUNNING", online.linux_anchor)

        rcu_core = by_name["RcuCore.InkernelBootEnded"]
        self.assertEqual(rcu_core.mapping_kind, "exact")
        self.assertEqual(rcu_core.confidence, "medium")
        self.assertIn("rcu_end_inkernel_boot", rcu_core.linux_anchor)
        self.assertIn("RcuCore object fact", rcu_core.notes)

        rcu_boundary = by_name["RcuBootEnd.Ready"]
        self.assertEqual(rcu_boundary.mapping_kind, "exact")
        self.assertEqual(rcu_boundary.confidence, "medium")
        self.assertIn("rcu_end_inkernel_boot", rcu_boundary.linux_anchor)
        self.assertIn("RCU boot-end boundary", rcu_boundary.notes)
        self.assertIn("reuses the RcuCore anchor", rcu_boundary.notes)

        sysctl = by_name["SysctlArgs.DeferredReady"]
        self.assertEqual(sysctl.mapping_kind, "exact")
        self.assertEqual(sysctl.confidence, "medium")
        self.assertIn("do_sysctl_args", sysctl.linux_anchor)
        self.assertIn("deferred finalize boundary", sysctl.notes)

    def test_runtime_core_object_rules_map_to_kernel_init_freeable(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=323,
                variant="RuntimeCorePhaseStarted",
                name="RuntimeCorePhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=324,
                variant="SchedulerSmpReady",
                name="Scheduler.SmpReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=325,
                variant="WorkqueueTopologyReady",
                name="Workqueue.TopologyReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=326,
                variant="AsyncCoreDeferredReady",
                name="AsyncCore.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=327,
                variant="PadataCoreDeferredReady",
                name="PadataCore.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=328,
                variant="PageAllocatorLateReady",
                name="PageAllocator.LateReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=329,
                variant="RuntimeCoreBoundaryReady",
                name="RuntimeCoreBoundary.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        for record in by_name.values():
            self.assertEqual(record.linux_file, "init/main.c")
            self.assertEqual(record.linux_symbol, "kernel_init_freeable")
            self.assertEqual(record.mapping_kind, "exact")
            self.assertEqual(record.confidence, "medium")

        scheduler = by_name["Scheduler.SmpReady"]
        self.assertIn("sched_init_smp", scheduler.linux_anchor)
        self.assertIn("object fact", scheduler.notes)
        self.assertIn("SmpBringupPhase.Ready", scheduler.notes)

        phase_started = by_name["RuntimeCorePhase.Started"]
        self.assertIn("sched_init_smp", phase_started.linux_anchor)
        self.assertIn("Preset starts", phase_started.notes)

        workqueue = by_name["Workqueue.TopologyReady"]
        self.assertIn("workqueue_init_topology", workqueue.linux_anchor)
        self.assertIn("topology object fact", workqueue.notes)

        async_core = by_name["AsyncCore.DeferredReady"]
        self.assertIn("async_init", async_core.linux_anchor)
        self.assertIn("deferred runtime-core boundary", async_core.notes)

        padata = by_name["PadataCore.DeferredReady"]
        self.assertIn("padata_init", padata.linux_anchor)
        self.assertIn("without expanding padata object details", padata.notes)

        page_allocator = by_name["PageAllocator.LateReady"]
        self.assertIn("page_alloc_init_late", page_allocator.linux_anchor)
        self.assertIn("PageAllocator late object fact", page_allocator.notes)

        boundary = by_name["RuntimeCoreBoundary.Ready"]
        self.assertIn("page_alloc_init_late", boundary.linux_anchor)
        self.assertIn("Runtime Core window end boundary", boundary.notes)
        self.assertIn("not an independent Linux object", boundary.notes)

    def test_initcall_core_object_rules_map_to_do_basic_setup(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=332,
                variant="CpusetSmpTrimmedReady",
                name="CpusetSmp.TrimmedReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=333,
                variant="DriverCoreDeferredReady",
                name="DriverCore.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=334,
                variant="IrqProcViewDeferredReady",
                name="IrqProcView.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=335,
                variant="CtorTableReady",
                name="CtorTable.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=344,
                variant="InitcallTableReady",
                name="InitcallTable.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=350,
                variant="InitcallBoundaryReady",
                name="InitcallBoundary.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        for record in by_name.values():
            self.assertEqual(record.linux_file, "init/main.c")
            self.assertEqual(record.linux_symbol, "do_basic_setup")
            self.assertEqual(record.mapping_kind, "exact")
            self.assertEqual(record.confidence, "medium")

        cpuset = by_name["CpusetSmp.TrimmedReady"]
        self.assertIn("cpuset_init_smp", cpuset.linux_anchor)
        self.assertIn("cpuset/cgroup trimmed/no-op object fact", cpuset.notes)

        driver = by_name["DriverCore.DeferredReady"]
        self.assertIn("driver_init", driver.linux_anchor)
        self.assertIn("driver core base/deferred boundary", driver.notes)
        self.assertIn("without expanding driver model subobjects", driver.notes)

        irq_proc = by_name["IrqProcView.DeferredReady"]
        self.assertIn("init_irq_proc", irq_proc.linux_anchor)
        self.assertIn("procfs IRQ view deferred boundary", irq_proc.notes)

        ctor = by_name["CtorTable.Ready"]
        self.assertIn("do_ctors", ctor.linux_anchor)
        self.assertIn("constructor table dispatch boundary", ctor.notes)

        initcall_table = by_name["InitcallTable.Ready"]
        self.assertIn("do_initcalls", initcall_table.linux_anchor)
        self.assertIn("initcall levels dispatcher/table object fact", initcall_table.notes)
        self.assertIn("not any individual initcall entry side effect", initcall_table.notes)

        boundary = by_name["InitcallBoundary.Ready"]
        self.assertIn("do_initcalls", boundary.linux_anchor)
        self.assertIn("do_basic_setup() end boundary", boundary.notes)
        self.assertIn("kunit_run_all_tests", boundary.notes)
        self.assertIn("not an independent Linux object", boundary.notes)

    def test_rootfs_tail_rules_map_to_kernel_init_freeable_and_prepare_namespace(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=352,
                variant="RootfsPhaseStarted",
                name="RootfsPhase.Started",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=353,
                variant="KUnitRuntimeTrimmedReady",
                name="KUnitRuntime.TrimmedReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=354,
                variant="InitramfsSyncDeferredReady",
                name="InitramfsSync.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=355,
                variant="RootfsConsoleDeferredReady",
                name="RootfsConsole.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=356,
                variant="RootfsPrepareNamespacePathsReady",
                name="RootfsPrepareNamespacePaths.Ready",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=357,
                variant="RamdiskExecuteCommandEaccessCheckpoint",
                name="RamdiskExecuteCommand.EaccessCheckpoint",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=358,
                variant="RootFSOnline",
                name="RootFS.Online",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=359,
                variant="IntegrityKeysDeferredReady",
                name="IntegrityKeys.DeferredReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=360,
                variant="RootfsBoundaryReady",
                name="RootfsBoundary.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        rootfs_started = by_name["RootfsPhase.Started"]
        self.assertEqual(rootfs_started.mapping_kind, "exact")
        self.assertEqual(rootfs_started.confidence, "high")
        self.assertIn("kunit_run_all_tests", rootfs_started.linux_anchor)
        self.assertIn("prepare_namespace pre-boundary", rootfs_started.notes)

        for name in (
            "KUnitRuntime.TrimmedReady",
            "InitramfsSync.DeferredReady",
            "RootfsConsole.DeferredReady",
            "RamdiskExecuteCommand.EaccessCheckpoint",
            "IntegrityKeys.DeferredReady",
            "RootfsBoundary.Ready",
        ):
            record = by_name[name]
            self.assertEqual(record.linux_file, "init/main.c")
            self.assertEqual(record.linux_symbol, "kernel_init_freeable")
            self.assertEqual(record.mapping_kind, "exact")
            self.assertEqual(record.confidence, "medium")

        kunit = by_name["KUnitRuntime.TrimmedReady"]
        self.assertIn("kunit_run_all_tests", kunit.linux_anchor)
        self.assertIn("trimmed/no-op fact", kunit.notes)

        initramfs = by_name["InitramfsSync.DeferredReady"]
        self.assertIn("wait_for_initramfs", initramfs.linux_anchor)
        self.assertIn("deferred initramfs synchronization boundary", initramfs.notes)

        console = by_name["RootfsConsole.DeferredReady"]
        self.assertIn("console_on_rootfs", console.linux_anchor)
        self.assertIn("deferred boundary", console.notes)

        paths = by_name["RootfsPrepareNamespacePaths.Ready"]
        self.assertEqual(paths.linux_file, "init/do_mounts.c")
        self.assertEqual(paths.linux_symbol, "prepare_namespace")
        self.assertEqual(paths.mapping_kind, "range")
        self.assertEqual(paths.confidence, "medium")
        self.assertIn("wait_for_device_probe", paths.linux_anchor)
        self.assertIn('init_chroot(".")', paths.linux_anchor)
        self.assertIn("path-classification interval", paths.notes)
        self.assertIn("devtmpfs classifications", paths.notes)

        ramdisk = by_name["RamdiskExecuteCommand.EaccessCheckpoint"]
        self.assertIn("init_eaccess", ramdisk.linux_anchor)
        self.assertIn("prepare_namespace", ramdisk.notes)

        rootfs = by_name["RootFS.Online"]
        self.assertEqual(rootfs.linux_file, "init/do_mounts.c")
        self.assertEqual(rootfs.linux_symbol, "prepare_namespace")
        self.assertEqual(rootfs.mapping_kind, "exact")
        self.assertEqual(rootfs.confidence, "medium")
        self.assertIn('init_chroot(".")', rootfs.linux_anchor)
        self.assertIn("RootFS object online fact", rootfs.notes)

        integrity = by_name["IntegrityKeys.DeferredReady"]
        self.assertIn("integrity_load_keys", integrity.linux_anchor)
        self.assertIn("deferred/trimmed boundary", integrity.notes)

        boundary = by_name["RootfsBoundary.Ready"]
        self.assertIn("integrity_load_keys", boundary.linux_anchor)
        self.assertIn("Rootfs end boundary", boundary.notes)
        self.assertIn("not an independent Linux object", boundary.notes)

    def test_user_boot_rules_map_to_linux_init_exec_anchors(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=374,
                variant="UserBootMainElfReady",
                name="UserBoot.MainElfReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=375,
                variant="UserBootInterpreterReady",
                name="UserBoot.InterpreterReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=376,
                variant="UserBootInitAttemptFailed",
                name="UserBoot.InitAttemptFailed",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=377,
                variant="UserBootAddressSpaceSetupStart",
                name="UserBoot.AddressSpaceSetupStart",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=378,
                variant="UserAppFlowEnterUserMode",
                name="UserAppFlow.EnterUserMode",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=379,
                variant="UserAddressSpaceReady",
                name="UserAddressSpace.Ready",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        main_elf = by_name["UserBoot.MainElfReady"]
        self.assertEqual(main_elf.linux_file, "fs/binfmt_elf.c")
        self.assertEqual(main_elf.linux_symbol, "load_elf_binary")
        self.assertEqual(main_elf.mapping_kind, "exact")
        self.assertEqual(main_elf.confidence, "high")
        self.assertIn("load_elf_phdrs", main_elf.linux_anchor)
        self.assertIn("Boot-time init exec", main_elf.notes)
        self.assertIn("kernel_execve", main_elf.notes)

        interpreter = by_name["UserBoot.InterpreterReady"]
        self.assertEqual(interpreter.mapping_kind, "exact")
        self.assertEqual(interpreter.confidence, "high")
        self.assertIn("interp_elf_phdata", interpreter.linux_anchor)
        self.assertIn("runtime UserExec", interpreter.notes)

        init_failed = by_name["UserBoot.InitAttemptFailed"]
        self.assertEqual(init_failed.linux_file, "init/main.c")
        self.assertEqual(init_failed.linux_symbol, "try_to_run_init_process")
        self.assertEqual(init_failed.mapping_kind, "range")
        self.assertEqual(init_failed.confidence, "medium")
        self.assertIn("run_init_process", init_failed.linux_anchor)
        self.assertIn("return ret", init_failed.linux_anchor)
        self.assertIn("stage/reason checkpoint", init_failed.notes)

        address_start = by_name["UserBoot.AddressSpaceSetupStart"]
        self.assertEqual(address_start.linux_file, "fs/binfmt_elf.c")
        self.assertEqual(address_start.linux_symbol, "load_elf_binary")
        self.assertEqual(address_start.mapping_kind, "exact")
        self.assertEqual(address_start.confidence, "medium")
        self.assertIn("begin_new_exec", address_start.linux_anchor)
        self.assertIn("UserBoot-specific", address_start.notes)

        enter_user = by_name["UserAppFlow.EnterUserMode"]
        self.assertEqual(enter_user.linux_file, "arch/riscv/kernel/entry.S")
        self.assertEqual(enter_user.linux_symbol, "ret_from_exception")
        self.assertEqual(enter_user.confidence, "medium")
        self.assertIn("sret", enter_user.linux_anchor)

        space_ready = by_name["UserAddressSpace.Ready"]
        self.assertEqual(space_ready.linux_file, "fs/exec.c")
        self.assertEqual(space_ready.linux_symbol, "exec_mmap")
        self.assertEqual(space_ready.confidence, "medium")
        self.assertIn("activate_mm", space_ready.linux_anchor)
        self.assertIn("UserAddressSpace object boundary", space_ready.notes)

    def test_user_mode_boundary_rules_map_to_linux_anchors(self) -> None:
        records = [
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=380,
                variant="SyscallTableExecveArgsReady",
                name="SyscallTable.ExecveArgsReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=381,
                variant="UserExecMainElfReady",
                name="UserExec.MainElfReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=382,
                variant="UserExecInterpreterReady",
                name="UserExec.InterpreterReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=383,
                variant="UserExecAddressSpaceReady",
                name="UserExec.AddressSpaceReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=384,
                variant="UserExecTrapFrameReady",
                name="UserExec.TrapFrameReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=385,
                variant="UserExecSatpReady",
                name="UserExec.SatpReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=386,
                variant="UserExecContextReplaced",
                name="UserExec.ContextReplaced",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=387,
                variant="UserExecSatpSwitched",
                name="UserExec.SatpSwitched",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=388,
                variant="UserExecReturnFrameReady",
                name="UserExec.ReturnFrameReady",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=389,
                variant="SyscallTableOpenAt",
                name="SyscallTable.OpenAt",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=390,
                variant="SyscallTableRead",
                name="SyscallTable.Read",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=391,
                variant="SyscallTableWrite",
                name="SyscallTable.Write",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=392,
                variant="SyscallTableWritev",
                name="SyscallTable.Writev",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=393,
                variant="SyscallTableClose",
                name="SyscallTable.Close",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=394,
                variant="SyscallTableNewFstatAt",
                name="SyscallTable.NewFstatAt",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=395,
                variant="SyscallTableSetTidAddress",
                name="SyscallTable.SetTidAddress",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=396,
                variant="SyscallTableClone",
                name="SyscallTable.Clone",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=397,
                variant="SyscallTableRtSigtimedwait",
                name="SyscallTable.RtSigtimedwait",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=398,
                variant="SyscallTableWait4",
                name="SyscallTable.Wait4",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=399,
                variant="UserChildParentWaitResumed",
                name="UserChild.ParentWaitResumed",
            ),
            map_linux_checkpoints.CheckpointInventoryRecord(
                index=400,
                variant="SyscallTableExit",
                name="SyscallTable.Exit",
            ),
        ]

        with tempfile.TemporaryDirectory() as tmp:
            mapped = map_linux_checkpoints.map_checkpoints(
                records,
                linux_tree=self._write_linux_fixture(tmp),
            )

        by_name = {record.checkpoint_name: record for record in mapped}
        self.assertEqual(by_name["SyscallTable.ExecveArgsReady"].linux_file, "fs/exec.c")
        self.assertIn("copy_strings(bprm->argc", by_name["SyscallTable.ExecveArgsReady"].linux_anchor)
        self.assertEqual(by_name["UserExec.MainElfReady"].linux_symbol, "load_elf_binary")
        self.assertIn("load_elf_phdrs", by_name["UserExec.InterpreterReady"].linux_anchor)
        self.assertEqual(by_name["UserExec.AddressSpaceReady"].mapping_kind, "range")
        self.assertEqual(by_name["UserExec.AddressSpaceReady"].confidence, "medium")
        self.assertEqual(by_name["UserExec.TrapFrameReady"].linux_file, "arch/riscv/kernel/process.c")
        self.assertEqual(by_name["UserExec.SatpReady"].linux_symbol, "exec_mmap")
        self.assertIn("exec_mmap", by_name["UserExec.ContextReplaced"].linux_anchor)
        self.assertEqual(by_name["UserExec.SatpSwitched"].linux_symbol, "ret_from_exception")
        self.assertIn("sret", by_name["UserExec.ReturnFrameReady"].linux_anchor)
        self.assertEqual(by_name["SyscallTable.OpenAt"].linux_symbol, "SYSCALL_DEFINE4(openat)")
        self.assertIn("do_sys_open", by_name["SyscallTable.OpenAt"].linux_anchor)
        self.assertEqual(by_name["SyscallTable.Read"].linux_symbol, "SYSCALL_DEFINE3(read)")
        self.assertIn("ksys_read", by_name["SyscallTable.Read"].linux_anchor)
        self.assertIn("ksys_write", by_name["SyscallTable.Write"].linux_anchor)
        self.assertIn("do_writev", by_name["SyscallTable.Writev"].linux_anchor)
        self.assertEqual(by_name["SyscallTable.Close"].linux_file, "fs/open.c")
        self.assertIn("file_close_fd", by_name["SyscallTable.Close"].linux_anchor)
        self.assertIn("vfs_fstatat", by_name["SyscallTable.NewFstatAt"].linux_anchor)
        self.assertIn("clear_child_tid", by_name["SyscallTable.SetTidAddress"].linux_anchor)
        self.assertEqual(by_name["SyscallTable.Clone"].linux_symbol, "kernel_clone")
        self.assertEqual(by_name["SyscallTable.Clone"].confidence, "medium")
        self.assertIn("conditional", by_name["SyscallTable.Clone"].notes)
        self.assertEqual(by_name["SyscallTable.RtSigtimedwait"].linux_file, "kernel/signal.c")
        self.assertIn("do_sigtimedwait", by_name["SyscallTable.RtSigtimedwait"].linux_anchor)
        self.assertIn("kernel_wait4", by_name["SyscallTable.Wait4"].linux_anchor)
        self.assertEqual(by_name["UserChild.ParentWaitResumed"].linux_symbol, "kernel_wait4")
        self.assertEqual(by_name["UserChild.ParentWaitResumed"].confidence, "medium")
        self.assertEqual(by_name["SyscallTable.Exit"].linux_symbol, "do_group_exit")
        self.assertEqual(by_name["SyscallTable.Exit"].confidence, "medium")

    def test_write_outputs_uses_fixed_record_fields(self) -> None:
        records = [
            map_linux_checkpoints.LinuxCheckpointMappingRecord(
                checkpoint_index=1,
                checkpoint_name="Demo.Ready",
                checkpoint_variant="DemoReady",
                linux_file="init/main.c",
                linux_symbol="start_kernel",
                linux_anchor="start_kernel() definition line 1",
                mapping_kind="exact",
                confidence="high",
                notes="fixture",
            )
        ]

        with tempfile.TemporaryDirectory() as tmp:
            json_path, markdown_path = map_linux_checkpoints.write_outputs(records, Path(tmp))
            rows = json.loads(json_path.read_text(encoding="utf-8"))
            markdown = markdown_path.read_text(encoding="utf-8")

        self.assertEqual(
            list(rows[0].keys()),
            [
                "checkpoint_index",
                "checkpoint_name",
                "checkpoint_variant",
                "linux_file",
                "linux_symbol",
                "linux_anchor",
                "mapping_kind",
                "confidence",
                "notes",
            ],
        )
        self.assertIn("| 1 | Demo.Ready | DemoReady | exact | high |", markdown)

    def test_check_mode_accepts_current_outputs(self) -> None:
        inventory = [
            {
                "index": 0,
                "variant": "KernelStarted",
                "name": "Kernel.Started",
            },
            {
                "index": 401,
                "variant": "KernelInitFlowPayloadHandoffCommitted",
                "name": "KernelInitFlow.PayloadHandoffCommitted",
            },
        ]

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            linux_tree = self._write_linux_fixture(tmp)
            inventory_path = tmp_path / "inventory.json"
            inventory_path.write_text(json.dumps(inventory), encoding="utf-8")
            records = map_linux_checkpoints.load_inventory(inventory_path)
            mapped = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)
            out_dir = tmp_path / "out"
            map_linux_checkpoints.write_outputs(mapped, out_dir)

            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                rc = map_linux_checkpoints.main(
                    [
                        "--input",
                        str(inventory_path),
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )

        self.assertEqual(rc, 0)
        self.assertIn("artifacts are current", stdout.getvalue())

    def test_check_mode_reports_drift_without_rewriting_outputs(self) -> None:
        inventory = [
            {
                "index": 0,
                "variant": "KernelStarted",
                "name": "Kernel.Started",
            }
        ]

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            linux_tree = self._write_linux_fixture(tmp)
            inventory_path = tmp_path / "inventory.json"
            inventory_path.write_text(json.dumps(inventory), encoding="utf-8")
            records = map_linux_checkpoints.load_inventory(inventory_path)
            mapped = map_linux_checkpoints.map_checkpoints(records, linux_tree=linux_tree)
            out_dir = tmp_path / "out"
            json_path, _markdown_path = map_linux_checkpoints.write_outputs(mapped, out_dir)
            stale_json = "[]\n"
            json_path.write_text(stale_json, encoding="utf-8")

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = map_linux_checkpoints.main(
                    [
                        "--input",
                        str(inventory_path),
                        "--linux-tree",
                        str(linux_tree),
                        "--out-dir",
                        str(out_dir),
                        "--check",
                    ]
                )

            json_after_check = json_path.read_text(encoding="utf-8")

        self.assertEqual(rc, 1)
        self.assertEqual(json_after_check, stale_json)
        self.assertIn("content differs", stderr.getvalue())

    def test_check_mode_requires_linux_tree(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            inventory_path = tmp_path / "inventory.json"
            inventory_path.write_text("[]", encoding="utf-8")

            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                rc = map_linux_checkpoints.main(
                    [
                        "--input",
                        str(inventory_path),
                        "--linux-tree",
                        str(tmp_path / "missing-linux"),
                        "--out-dir",
                        str(tmp_path / "out"),
                        "--check",
                    ]
                )

        self.assertEqual(rc, 1)
        self.assertIn("Linux reference tree is missing", stderr.getvalue())

    @unittest.skipUnless(
        map_linux_checkpoints.DEFAULT_LINUX_TREE.is_dir()
        and map_linux_checkpoints.DEFAULT_INVENTORY.is_file(),
        "default ../linux-6.12 tree or generated checkpoint inventory is not available",
    )
    def test_real_linux_tree_smoke_locates_expected_symbols(self) -> None:
        inventory = map_linux_checkpoints.load_inventory(map_linux_checkpoints.DEFAULT_INVENTORY)
        mapped = map_linux_checkpoints.map_checkpoints(inventory)
        by_name = {record.checkpoint_name: record for record in mapped}

        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_file, "arch/riscv/kernel/head.S")
        self.assertEqual(by_name["EntryPreludePhase.Started"].linux_symbol, "_start")
        self.assertIn("tail start_kernel", by_name["EntryPreludePhase.Ready"].linux_anchor)
        self.assertNotEqual(by_name["EarlyVm.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["TrampolineVm.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["RawDtb.Ready"].mapping_kind, "unmapped")
        self.assertNotEqual(by_name["FixMap.Ready"].mapping_kind, "unmapped")
        self.assertEqual(by_name["Kernel.Started"].linux_symbol, "start_kernel")
        self.assertEqual(by_name["MmCoreInitPhase.Ready"].linux_file, "mm/mm_init.c")
        self.assertEqual(by_name["SchedInitPhase.Ready"].linux_file, "kernel/sched/core.c")
        self.assertEqual(by_name["BootInitRestInitPhase.Ready"].linux_symbol, "rest_init")
        self.assertEqual(by_name["RootfsPhase.Ready"].linux_symbol, "prepare_namespace")
        self.assertEqual(by_name["KUnitRuntime.TrimmedReady"].linux_symbol, "kernel_init_freeable")
        self.assertEqual(by_name["InitramfsSync.DeferredReady"].linux_symbol, "kernel_init_freeable")
        self.assertEqual(by_name["RootfsConsole.DeferredReady"].linux_symbol, "kernel_init_freeable")
        self.assertEqual(by_name["RootfsPrepareNamespacePaths.Ready"].mapping_kind, "range")
        self.assertEqual(
            by_name["RamdiskExecuteCommand.EaccessCheckpoint"].confidence,
            "medium",
        )
        self.assertEqual(by_name["RootFS.Online"].linux_symbol, "prepare_namespace")
        self.assertEqual(by_name["IntegrityKeys.DeferredReady"].linux_symbol, "kernel_init_freeable")
        self.assertEqual(by_name["RootfsBoundary.Ready"].confidence, "medium")
        self.assertEqual(
            by_name["KernelInitFlow.PayloadHandoffCommitted"].linux_symbol,
            "run_init_process",
        )
        self.assertIn(
            "definition",
            by_name["KernelInitFlow.PayloadHandoffCommitted"].linux_anchor,
        )
        self.assertEqual(by_name["SyscallTable.Read"].linux_symbol, "SYSCALL_DEFINE3(read)")
        self.assertEqual(by_name["SyscallTable.Write"].linux_symbol, "SYSCALL_DEFINE3(write)")
        self.assertEqual(by_name["SyscallTable.Clone"].linux_symbol, "kernel_clone")
        self.assertEqual(by_name["SyscallTable.Clone"].confidence, "medium")
        self.assertEqual(by_name["AsyncFullSync.DeferredReady"].linux_symbol, "kernel_init")
        self.assertEqual(
            by_name["SystemState.FreeingInitmemCheckpoint"].confidence,
            "high",
        )
        self.assertEqual(by_name["InitMemoryCleanup.DeferredReady"].linux_symbol, "kernel_init")
        self.assertEqual(
            by_name["KernelMappingProtection.DeferredReady"].linux_symbol,
            "kernel_init",
        )
        self.assertEqual(by_name["PtiFinalize.TrimmedReady"].confidence, "medium")
        self.assertEqual(by_name["SystemState.Online"].confidence, "high")
        self.assertEqual(by_name["RcuCore.InkernelBootEnded"].linux_symbol, "kernel_init")
        self.assertEqual(by_name["RcuBootEnd.Ready"].confidence, "medium")
        self.assertEqual(by_name["SysctlArgs.DeferredReady"].linux_symbol, "kernel_init")
        self.assertEqual(by_name["UserBoot.MainElfReady"].linux_symbol, "load_elf_binary")
        self.assertEqual(by_name["UserBoot.InitAttemptFailed"].mapping_kind, "range")
        self.assertEqual(by_name["UserBoot.AddressSpaceSetupStart"].confidence, "medium")
        self.assertEqual(by_name["UserAppFlow.EnterUserMode"].linux_symbol, "ret_from_exception")
        self.assertEqual(by_name["UserAddressSpace.Ready"].linux_symbol, "exec_mmap")
        self.assertEqual(by_name["UserExec.AddressSpaceReady"].mapping_kind, "range")
        self.assertEqual(by_name["UserExec.TrapFrameReady"].linux_symbol, "start_thread")
        self.assertEqual(by_name["UserChild.ParentWaitResumed"].linux_symbol, "kernel_wait4")


if __name__ == "__main__":
    unittest.main()
