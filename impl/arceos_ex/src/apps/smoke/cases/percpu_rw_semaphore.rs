use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        percpu_rw_semaphore::{
            PerCpuRwSemaphore, PerCpuRwSemaphoreExtState, PerCpuRwSemaphoreInitKind,
            PerCpuRwSemaphoreOwner, PerCpuRwSemaphoreReadOutcome, PerCpuRwSemaphoreWriteOutcome,
        },
        state::State,
        task::TaskRef,
    },
};

const BOOT_CPU_ID: usize = 0;
const SCHEDULE_ATTEMPTS: usize = 4;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SetupReadyScenario::new());
    suite.scenario(&mut UnpreparedOperationsScenario::new());
    suite.scenario(&mut ReadLockUnlockScenario::new());
    suite.scenario(&mut WriterBlockDrainScenario::new());
    suite.scenario(&mut CooperativeWriterScenario::new());
    suite.result()
}

struct RwsemFixture {
    lock: PerCpuRwSemaphore,
}

impl RwsemFixture {
    const fn new() -> Self {
        Self {
            lock: PerCpuRwSemaphore::new_static(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("preset rwsem", self.lock.preset_static());
        assertions.assert_ok("setup rwsem", self.lock.setup());
    }

    fn unlock_reader_if_held(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
        assertions: &mut SmokeAssertions,
    ) {
        if self.lock.active_readers() != 0 {
            let _ = self
                .lock
                .read_unlock_owner(owner, BOOT_CPU_ID)
                .map_err(|_| {
                    assertions.assert("teardown read unlock", false);
                });
        }
    }

    fn unlock_writer_if_held(
        &mut self,
        owner: PerCpuRwSemaphoreOwner,
        assertions: &mut SmokeAssertions,
    ) {
        if self.lock.writer() == owner {
            assertions.assert_ok("teardown write unlock", self.lock.write_unlock_owner(owner));
        }
    }
}

struct SetupReadyScenario {
    fixture: RwsemFixture,
}

impl SetupReadyScenario {
    const fn new() -> Self {
        Self {
            fixture: RwsemFixture::new(),
        }
    }
}

impl SmokeScenario for SetupReadyScenario {
    fn name(&self) -> &'static str {
        "percpu_rwsem.setup_ready"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("ready", self.fixture.lock.ready());
        assertions.assert("state ready", self.fixture.lock.state() == State::Ready);
        assertions.assert(
            "static initializer",
            self.fixture.lock.init_kind() == PerCpuRwSemaphoreInitKind::StaticInitializer,
        );
        assertions.assert(
            "readers fast",
            self.fixture.lock.ext_state() == PerCpuRwSemaphoreExtState::ReadersFast,
        );
        assertions.assert("storage bound", self.fixture.lock.storage_bound());
        assertions.assert(
            "percpu read counter bound",
            self.fixture.lock.percpu_read_counter_bound(),
        );
        assertions.assert(
            "boot cpu read available",
            self.fixture.lock.boot_cpu_read_available(),
        );
        assertions.assert("wait queue ready", self.fixture.lock.wait_queue_ready());
        assertions.assert("block clear", self.fixture.lock.block_flag_clear());
        assertions.assert(
            "rcu sync ready",
            self.fixture.lock.rcu_sync().state() == State::Ready,
        );
        assertions.assert("rcu idle", self.fixture.lock.rcu_sync().idle());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct UnpreparedOperationsScenario {
    fixture: RwsemFixture,
}

impl UnpreparedOperationsScenario {
    const fn new() -> Self {
        Self {
            fixture: RwsemFixture::new(),
        }
    }
}

impl SmokeScenario for UnpreparedOperationsScenario {
    fn name(&self) -> &'static str {
        "percpu_rwsem.unprepared_operations"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "read before setup",
            self.fixture
                .lock
                .read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID),
        );
        assertions.assert_fail(
            "write before setup",
            self.fixture
                .lock
                .write_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct ReadLockUnlockScenario {
    fixture: RwsemFixture,
}

impl ReadLockUnlockScenario {
    const fn new() -> Self {
        Self {
            fixture: RwsemFixture::new(),
        }
    }
}

impl SmokeScenario for ReadLockUnlockScenario {
    fn name(&self) -> &'static str {
        "percpu_rwsem.read_lock_unlock"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "read acquired",
            self.fixture
                .lock
                .read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID)
                == Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast),
        );
        assertions.assert("reader count", self.fixture.lock.active_readers() == 1);
        assertions.assert_ok(
            "read unlock",
            self.fixture
                .lock
                .read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID),
        );
        assertions.assert("reader drained", self.fixture.lock.active_readers() == 0);
        assertions.assert("read lock count", self.fixture.lock.read_lock_count() == 1);
        assertions.assert(
            "read unlock count",
            self.fixture.lock.read_unlock_count() == 1,
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_reader_if_held(PerCpuRwSemaphoreOwner::KernelInitTask, assertions);
    }
}

struct WriterBlockDrainScenario {
    fixture: RwsemFixture,
}

impl WriterBlockDrainScenario {
    const fn new() -> Self {
        Self {
            fixture: RwsemFixture::new(),
        }
    }
}

impl SmokeScenario for WriterBlockDrainScenario {
    fn name(&self) -> &'static str {
        "percpu_rwsem.writer_block_drain"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "reader acquired",
            self.fixture
                .lock
                .read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID)
                == Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast),
        );
        assertions.assert(
            "writer blocked",
            self.fixture
                .lock
                .write_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask)
                == Ok(PerCpuRwSemaphoreWriteOutcome::Blocked),
        );
        assertions.assert(
            "try read blocked",
            self.fixture
                .lock
                .read_try_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID)
                == Ok(PerCpuRwSemaphoreReadOutcome::Blocked),
        );
        assertions.assert_ok(
            "reader unlock",
            self.fixture
                .lock
                .read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID),
        );
        assertions.assert(
            "reader drain count",
            self.fixture.lock.reader_drain_count() == 1,
        );
        assertions.assert(
            "writer retry acquired",
            self.fixture
                .lock
                .write_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask)
                == Ok(PerCpuRwSemaphoreWriteOutcome::Acquired),
        );
        assertions.assert(
            "writer active",
            self.fixture.lock.ext_state() == PerCpuRwSemaphoreExtState::WriterActive,
        );
        assertions.assert_ok(
            "writer unlock",
            self.fixture
                .lock
                .write_unlock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask),
        );
        assertions.assert("readers fast again", self.fixture.lock.readers_fast());
        assertions.assert(
            "rcu grace period",
            self.fixture.lock.rcu_sync().grace_period_count() == 1,
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_reader_if_held(PerCpuRwSemaphoreOwner::KernelInitTask, assertions);
        self.fixture
            .unlock_writer_if_held(PerCpuRwSemaphoreOwner::SmokeRwsemTask, assertions);
    }
}

struct CooperativeWriterScenario;

impl CooperativeWriterScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for CooperativeWriterScenario {
    fn name(&self) -> &'static str {
        "percpu_rwsem.cooperative_writer"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        shared.reset();
        assertions.assert_ok("preset shared rwsem", shared.lock.preset_static());
        assertions.assert_ok("setup shared rwsem", shared.lock.setup());

        let ctx = context();
        assertions.assert("scheduler online", ctx.scheduler.state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.boot_cpu_current_task.current_is_kernel_init(),
        );
        assertions.assert_ok(
            "setup smoke rwsem task",
            ctx.setup_smoke_rwsem_task(smoke_rwsem_task_entry),
        );
        assertions.assert_ok("enqueue smoke rwsem task", ctx.enqueue_smoke_rwsem_task());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        {
            let shared = shared_state();
            assertions.assert(
                "kernel reader acquired",
                shared
                    .lock
                    .read_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID)
                    == Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast),
            );
        }

        assertions.assert_ok("schedule writer task", context().schedule_current());

        {
            let shared = shared_state();
            assertions.assert("task entry ran", shared.task_entry_ran);
            assertions.assert("task writer blocked", shared.task_writer_blocked);
            assertions.assert("task yielded", shared.task_yielded_after_block);
            assertions.assert(
                "writer blocked count",
                shared.lock.writer_blocked_count() == 1,
            );
            assertions.assert(
                "current returned kernel init",
                context().boot_cpu_current_task.current() == TaskRef::KERNEL_INIT,
            );
            assertions.assert(
                "try read blocked",
                shared
                    .lock
                    .read_try_lock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID)
                    == Ok(PerCpuRwSemaphoreReadOutcome::Blocked),
            );
            assertions.assert_ok(
                "kernel reader unlock",
                shared
                    .lock
                    .read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID),
            );
            assertions.assert("reader drain", shared.lock.reader_drain_count() == 1);
        }

        let mut attempts = 0usize;
        while attempts < SCHEDULE_ATTEMPTS {
            if shared_state().task_completed {
                break;
            }
            assertions.assert_ok("schedule writer retry", context().schedule_current());
            attempts += 1;
        }

        let shared = shared_state();
        assertions.assert("bounded completion", attempts < SCHEDULE_ATTEMPTS);
        assertions.assert("task writer acquired", shared.task_writer_acquired);
        assertions.assert("task writer unlocked", shared.task_writer_unlocked);
        assertions.assert(
            "reader blocked while writer",
            shared.reader_blocked_while_writer,
        );
        assertions.assert("task reader recovered", shared.task_reader_recovered);
        assertions.assert("task completed", shared.task_completed);
        assertions.assert("final ready", shared.lock.ready());
        assertions.assert("write lock count", shared.lock.write_lock_count() == 1);
        assertions.assert("write unlock count", shared.lock.write_unlock_count() == 1);
        assertions.assert("read blocked count", shared.lock.read_blocked_count() >= 1);
        assertions.assert(
            "smoke rwsem task yielded",
            context().scheduler.smoke_rwsem_task().yielded_back(),
        );
        assertions.assert(
            "smoke rwsem task dequeued",
            !context()
                .scheduler
                .boot_cpu_owned_scheduler_view(&context().cpu_group)
                .map(|view| {
                    view.runqueue_contains_task_id(context().scheduler.smoke_rwsem_task().task_id())
                })
                .unwrap_or(false),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        if shared.lock.active_readers() != 0 {
            let _ = shared
                .lock
                .read_unlock_owner(PerCpuRwSemaphoreOwner::KernelInitTask, BOOT_CPU_ID);
            let _ = shared
                .lock
                .read_unlock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask, BOOT_CPU_ID);
        }
        if shared.lock.writer() == PerCpuRwSemaphoreOwner::SmokeRwsemTask {
            assertions.assert_ok(
                "teardown writer unlock",
                shared
                    .lock
                    .write_unlock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask),
            );
        }
    }
}

struct RwsemSmokeShared {
    lock: PerCpuRwSemaphore,
    task_entry_ran: bool,
    task_writer_blocked: bool,
    task_yielded_after_block: bool,
    task_writer_acquired: bool,
    reader_blocked_while_writer: bool,
    task_writer_unlocked: bool,
    task_reader_recovered: bool,
    task_completed: bool,
}

impl RwsemSmokeShared {
    const fn new() -> Self {
        Self {
            lock: PerCpuRwSemaphore::new_static(),
            task_entry_ran: false,
            task_writer_blocked: false,
            task_yielded_after_block: false,
            task_writer_acquired: false,
            reader_blocked_while_writer: false,
            task_writer_unlocked: false,
            task_reader_recovered: false,
            task_completed: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

static mut RWSEM_SMOKE_SHARED: RwsemSmokeShared = RwsemSmokeShared::new();

fn shared_state() -> &'static mut RwsemSmokeShared {
    // SAFETY: smoke runs on the boot CPU with cooperative task switches. This
    // state models the shared object observed by the two smoke tasks.
    unsafe { &mut *core::ptr::addr_of_mut!(RWSEM_SMOKE_SHARED) }
}

extern "C" fn smoke_rwsem_task_entry() -> ! {
    if context().mark_smoke_rwsem_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_entry_ran = true;

    match shared_state()
        .lock
        .write_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask)
    {
        Ok(PerCpuRwSemaphoreWriteOutcome::Blocked) => {
            shared_state().task_writer_blocked = true;
        }
        Ok(PerCpuRwSemaphoreWriteOutcome::Acquired) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    shared_state().task_yielded_after_block = true;
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    match shared_state()
        .lock
        .write_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask)
    {
        Ok(PerCpuRwSemaphoreWriteOutcome::Acquired) => {
            shared_state().task_writer_acquired = true;
        }
        Ok(PerCpuRwSemaphoreWriteOutcome::Blocked) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    match shared_state()
        .lock
        .read_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask, BOOT_CPU_ID)
    {
        Ok(PerCpuRwSemaphoreReadOutcome::Blocked) => {
            shared_state().reader_blocked_while_writer = true;
        }
        Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    if shared_state()
        .lock
        .write_unlock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask)
        .is_err()
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_writer_unlocked = true;

    match shared_state()
        .lock
        .read_lock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask, BOOT_CPU_ID)
    {
        Ok(PerCpuRwSemaphoreReadOutcome::AcquiredFast) => {
            shared_state().task_reader_recovered = true;
        }
        Ok(PerCpuRwSemaphoreReadOutcome::Blocked) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }
    if shared_state()
        .lock
        .read_unlock_owner(PerCpuRwSemaphoreOwner::SmokeRwsemTask, BOOT_CPU_ID)
        .is_err()
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_completed = true;

    if context().mark_smoke_rwsem_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().dequeue_smoke_rwsem_task().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    crate::arch::riscv64::sbi::system_shutdown()
}
