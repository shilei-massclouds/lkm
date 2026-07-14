use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        cpu_control::CurrentTaskRef,
        rwlock::{
            RwLock, RwLockExtState, RwLockInitKind, RwLockOwner, RwLockReadOutcome,
            RwLockWriteOutcome,
        },
        state::State,
    },
};

const SCHEDULE_ATTEMPTS: usize = 4;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SetupReadyScenario::new());
    suite.scenario(&mut UnpreparedOperationsScenario::new());
    suite.scenario(&mut ReadShareScenario::new());
    suite.scenario(&mut WriteExclusiveScenario::new());
    suite.scenario(&mut TryLockScenario::new());
    suite.scenario(&mut CooperativeWriterScenario::new());
    suite.result()
}

struct RwLockFixture {
    lock: RwLock,
}

impl RwLockFixture {
    const fn new() -> Self {
        Self {
            lock: RwLock::new_static(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("preset rwlock", self.lock.preset_static());
        assertions.assert_ok("setup rwlock", self.lock.setup());
    }

    fn unlock_reader_if_held(&mut self, owner: RwLockOwner) {
        if self.lock.active_readers() != 0 {
            let _ = self.lock.read_unlock_owner(owner);
        }
    }

    fn unlock_writer_if_held(&mut self, owner: RwLockOwner, assertions: &mut SmokeAssertions) {
        if self.lock.writer() == owner {
            assertions.assert_ok("teardown write unlock", self.lock.write_unlock_owner(owner));
        }
    }
}

struct SetupReadyScenario {
    fixture: RwLockFixture,
}

impl SetupReadyScenario {
    const fn new() -> Self {
        Self {
            fixture: RwLockFixture::new(),
        }
    }
}

impl SmokeScenario for SetupReadyScenario {
    fn name(&self) -> &'static str {
        "rwlock.setup_ready"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("ready", self.fixture.lock.ready());
        assertions.assert("state ready", self.fixture.lock.state() == State::Ready);
        assertions.assert(
            "static initializer",
            self.fixture.lock.init_kind() == RwLockInitKind::StaticInitializer,
        );
        assertions.assert(
            "unlocked state",
            self.fixture.lock.ext_state() == RwLockExtState::Unlocked,
        );
        assertions.assert("storage bound", self.fixture.lock.storage_bound());
        assertions.assert(
            "raw lock internal",
            self.fixture.lock.arch_raw_lock_internal(),
        );
        assertions.assert(
            "debug deferred",
            self.fixture.lock.debug_lockdep_internal_deferred(),
        );
        assertions.assert("no readers", self.fixture.lock.active_readers() == 0);
        assertions.assert("no writer", self.fixture.lock.writer() == RwLockOwner::None);
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct UnpreparedOperationsScenario {
    fixture: RwLockFixture,
}

impl UnpreparedOperationsScenario {
    const fn new() -> Self {
        Self {
            fixture: RwLockFixture::new(),
        }
    }
}

impl SmokeScenario for UnpreparedOperationsScenario {
    fn name(&self) -> &'static str {
        "rwlock.unprepared_operations"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "read before setup",
            self.fixture
                .lock
                .read_lock_owner(RwLockOwner::KernelInitTask),
        );
        assertions.assert_fail(
            "write before setup",
            self.fixture
                .lock
                .write_lock_owner(RwLockOwner::KernelInitTask),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct ReadShareScenario {
    fixture: RwLockFixture,
}

impl ReadShareScenario {
    const fn new() -> Self {
        Self {
            fixture: RwLockFixture::new(),
        }
    }
}

impl SmokeScenario for ReadShareScenario {
    fn name(&self) -> &'static str {
        "rwlock.read_share"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "first reader acquired",
            self.fixture
                .lock
                .read_lock_owner(RwLockOwner::KernelInitTask)
                == Ok(RwLockReadOutcome::Acquired),
        );
        assertions.assert(
            "second reader shared",
            self.fixture
                .lock
                .read_lock_owner(RwLockOwner::SmokeRwLockTask)
                == Ok(RwLockReadOutcome::Shared),
        );
        assertions.assert(
            "read held",
            self.fixture.lock.ext_state() == RwLockExtState::ReadHeld,
        );
        assertions.assert("two readers", self.fixture.lock.active_readers() == 2);
        assertions.assert_ok(
            "first reader unlock",
            self.fixture
                .lock
                .read_unlock_owner(RwLockOwner::KernelInitTask),
        );
        assertions.assert(
            "one reader remains",
            self.fixture.lock.active_readers() == 1,
        );
        assertions.assert_ok(
            "second reader unlock",
            self.fixture
                .lock
                .read_unlock_owner(RwLockOwner::SmokeRwLockTask),
        );
        assertions.assert("unlocked", self.fixture.lock.unlocked());
        assertions.assert("read lock count", self.fixture.lock.read_lock_count() == 2);
        assertions.assert(
            "read unlock count",
            self.fixture.lock.read_unlock_count() == 2,
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_reader_if_held(RwLockOwner::KernelInitTask);
        self.fixture
            .unlock_reader_if_held(RwLockOwner::SmokeRwLockTask);
    }
}

struct WriteExclusiveScenario {
    fixture: RwLockFixture,
}

impl WriteExclusiveScenario {
    const fn new() -> Self {
        Self {
            fixture: RwLockFixture::new(),
        }
    }
}

impl SmokeScenario for WriteExclusiveScenario {
    fn name(&self) -> &'static str {
        "rwlock.write_exclusive"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "writer acquired",
            self.fixture
                .lock
                .write_lock_owner(RwLockOwner::KernelInitTask)
                == Ok(RwLockWriteOutcome::Acquired),
        );
        assertions.assert(
            "write held",
            self.fixture.lock.ext_state() == RwLockExtState::WriteHeld,
        );
        assertions.assert(
            "reader blocked",
            self.fixture
                .lock
                .read_lock_owner(RwLockOwner::SmokeRwLockTask)
                == Ok(RwLockReadOutcome::Blocked),
        );
        assertions.assert(
            "writer blocked",
            self.fixture
                .lock
                .write_lock_owner(RwLockOwner::SmokeRwLockTask)
                == Ok(RwLockWriteOutcome::Blocked),
        );
        assertions.assert_ok(
            "writer unlock",
            self.fixture
                .lock
                .write_unlock_owner(RwLockOwner::KernelInitTask),
        );
        assertions.assert("unlocked", self.fixture.lock.unlocked());
        assertions.assert(
            "write lock count",
            self.fixture.lock.write_lock_count() == 1,
        );
        assertions.assert(
            "write unlock count",
            self.fixture.lock.write_unlock_count() == 1,
        );
        assertions.assert(
            "read blocked count",
            self.fixture.lock.read_blocked_count() == 1,
        );
        assertions.assert(
            "write blocked count",
            self.fixture.lock.write_blocked_count() == 1,
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_writer_if_held(RwLockOwner::KernelInitTask, assertions);
        self.fixture
            .unlock_writer_if_held(RwLockOwner::SmokeRwLockTask, assertions);
    }
}

struct TryLockScenario {
    fixture: RwLockFixture,
}

impl TryLockScenario {
    const fn new() -> Self {
        Self {
            fixture: RwLockFixture::new(),
        }
    }
}

impl SmokeScenario for TryLockScenario {
    fn name(&self) -> &'static str {
        "rwlock.trylock"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "read try acquired",
            self.fixture
                .lock
                .read_try_lock_owner(RwLockOwner::KernelInitTask)
                == Ok(RwLockReadOutcome::Acquired),
        );
        assertions.assert(
            "write try blocked",
            self.fixture
                .lock
                .write_try_lock_owner(RwLockOwner::SmokeRwLockTask)
                == Ok(RwLockWriteOutcome::Blocked),
        );
        assertions.assert_ok(
            "reader unlock",
            self.fixture
                .lock
                .read_unlock_owner(RwLockOwner::KernelInitTask),
        );
        assertions.assert(
            "write try acquired",
            self.fixture
                .lock
                .write_try_lock_owner(RwLockOwner::SmokeRwLockTask)
                == Ok(RwLockWriteOutcome::Acquired),
        );
        assertions.assert(
            "read try blocked",
            self.fixture
                .lock
                .read_try_lock_owner(RwLockOwner::KernelInitTask)
                == Ok(RwLockReadOutcome::Blocked),
        );
        assertions.assert_ok(
            "writer unlock",
            self.fixture
                .lock
                .write_unlock_owner(RwLockOwner::SmokeRwLockTask),
        );
        assertions.assert("read try count", self.fixture.lock.read_try_count() == 2);
        assertions.assert(
            "read try failed count",
            self.fixture.lock.read_try_failed_count() == 1,
        );
        assertions.assert("write try count", self.fixture.lock.write_try_count() == 2);
        assertions.assert(
            "write try failed count",
            self.fixture.lock.write_try_failed_count() == 1,
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_reader_if_held(RwLockOwner::KernelInitTask);
        self.fixture
            .unlock_writer_if_held(RwLockOwner::SmokeRwLockTask, assertions);
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
        "rwlock.cooperative_writer"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        shared.reset();
        assertions.assert_ok("preset shared rwlock", shared.lock.preset_static());
        assertions.assert_ok("setup shared rwlock", shared.lock.setup());

        let ctx = context();
        assertions.assert("scheduler online", ctx.scheduler.state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.boot_cpu_current_task.current_is_kernel_init(),
        );
        assertions.assert_ok(
            "setup smoke rwlock task",
            ctx.setup_smoke_rwlock_task(smoke_rwlock_task_entry),
        );
        assertions.assert_ok("enqueue smoke rwlock task", ctx.enqueue_smoke_rwlock_task());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        {
            let shared = shared_state();
            assertions.assert(
                "kernel reader acquired",
                shared.lock.read_lock_owner(RwLockOwner::KernelInitTask)
                    == Ok(RwLockReadOutcome::Acquired),
            );
        }

        assertions.assert_ok("schedule writer task", context().schedule_current());

        {
            let shared = shared_state();
            assertions.assert("task entry ran", shared.task_entry_ran);
            assertions.assert("task writer blocked", shared.task_writer_blocked);
            assertions.assert("task yielded", shared.task_yielded_after_block);
            assertions.assert(
                "write blocked count",
                shared.lock.write_blocked_count() == 1,
            );
            assertions.assert(
                "current returned kernel init",
                context().boot_cpu_current_task.current() == CurrentTaskRef::KernelInit,
            );
            assertions.assert_ok(
                "kernel reader unlock",
                shared.lock.read_unlock_owner(RwLockOwner::KernelInitTask),
            );
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
        assertions.assert(
            "reader blocked while writer",
            shared.reader_blocked_while_writer,
        );
        assertions.assert("task writer unlocked", shared.task_writer_unlocked);
        assertions.assert("task reader recovered", shared.task_reader_recovered);
        assertions.assert("task completed", shared.task_completed);
        assertions.assert("final ready", shared.lock.ready());
        assertions.assert("write lock count", shared.lock.write_lock_count() == 1);
        assertions.assert("write unlock count", shared.lock.write_unlock_count() == 1);
        assertions.assert("read blocked count", shared.lock.read_blocked_count() >= 1);
        assertions.assert(
            "smoke rwlock task yielded",
            context().scheduler.smoke_rwlock_task().yielded_back(),
        );
        assertions.assert(
            "smoke rwlock task dequeued",
            !context()
                .scheduler
                .boot_cpu_owned_scheduler_view(&context().cpu_group)
                .map(|view| {
                    view.runqueue_contains_task_id(
                        context().scheduler.smoke_rwlock_task().task_id(),
                    )
                })
                .unwrap_or(false),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        if shared.lock.active_readers() != 0 {
            let _ = shared.lock.read_unlock_owner(RwLockOwner::KernelInitTask);
            let _ = shared.lock.read_unlock_owner(RwLockOwner::SmokeRwLockTask);
        }
        if shared.lock.writer() == RwLockOwner::SmokeRwLockTask {
            assertions.assert_ok(
                "teardown writer unlock",
                shared.lock.write_unlock_owner(RwLockOwner::SmokeRwLockTask),
            );
        }
    }
}

struct RwLockSmokeShared {
    lock: RwLock,
    task_entry_ran: bool,
    task_writer_blocked: bool,
    task_yielded_after_block: bool,
    task_writer_acquired: bool,
    reader_blocked_while_writer: bool,
    task_writer_unlocked: bool,
    task_reader_recovered: bool,
    task_completed: bool,
}

impl RwLockSmokeShared {
    const fn new() -> Self {
        Self {
            lock: RwLock::new_static(),
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

static mut RWLOCK_SMOKE_SHARED: RwLockSmokeShared = RwLockSmokeShared::new();

fn shared_state() -> &'static mut RwLockSmokeShared {
    // SAFETY: smoke runs on the boot CPU with cooperative task switches. This
    // state models the shared object observed by the two smoke tasks.
    unsafe { &mut *core::ptr::addr_of_mut!(RWLOCK_SMOKE_SHARED) }
}

extern "C" fn smoke_rwlock_task_entry() -> ! {
    if context().mark_smoke_rwlock_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_entry_ran = true;

    match shared_state()
        .lock
        .write_lock_owner(RwLockOwner::SmokeRwLockTask)
    {
        Ok(RwLockWriteOutcome::Blocked) => {
            shared_state().task_writer_blocked = true;
        }
        Ok(RwLockWriteOutcome::Acquired) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    shared_state().task_yielded_after_block = true;
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    match shared_state()
        .lock
        .write_lock_owner(RwLockOwner::SmokeRwLockTask)
    {
        Ok(RwLockWriteOutcome::Acquired) => {
            shared_state().task_writer_acquired = true;
        }
        Ok(RwLockWriteOutcome::Blocked) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    match shared_state()
        .lock
        .read_lock_owner(RwLockOwner::SmokeRwLockTask)
    {
        Ok(RwLockReadOutcome::Blocked) => {
            shared_state().reader_blocked_while_writer = true;
        }
        Ok(RwLockReadOutcome::Acquired) | Ok(RwLockReadOutcome::Shared) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }

    if shared_state()
        .lock
        .write_unlock_owner(RwLockOwner::SmokeRwLockTask)
        .is_err()
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_writer_unlocked = true;

    match shared_state()
        .lock
        .read_lock_owner(RwLockOwner::SmokeRwLockTask)
    {
        Ok(RwLockReadOutcome::Acquired) => {
            shared_state().task_reader_recovered = true;
        }
        Ok(RwLockReadOutcome::Shared) | Ok(RwLockReadOutcome::Blocked) | Err(_) => {
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }
    if shared_state()
        .lock
        .read_unlock_owner(RwLockOwner::SmokeRwLockTask)
        .is_err()
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_completed = true;

    if context().mark_smoke_rwlock_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().dequeue_smoke_rwlock_task().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    crate::arch::riscv64::sbi::system_shutdown()
}
