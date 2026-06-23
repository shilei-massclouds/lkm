use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        cpu_control::CurrentTaskRef,
        mutex::{Mutex, MutexInitKind, MutexLockOutcome, MutexOwner},
        state::State,
    },
};

const SCHEDULE_ATTEMPTS: usize = 4;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut SetupReadyScenario::new());
    suite.scenario(&mut UnpreparedOperationsScenario::new());
    suite.scenario(&mut LockUnlockScenario::new());
    suite.scenario(&mut ContendedWakeScenario::new());
    suite.scenario(&mut CooperativeContentionScenario::new());
    suite.result()
}

struct MutexFixture {
    lock: Mutex,
}

impl MutexFixture {
    const fn new() -> Self {
        Self {
            lock: Mutex::new_static(),
        }
    }

    fn setup_ready(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_ok("preset static mutex", self.lock.preset_static());
        assertions.assert_ok("setup mutex", self.lock.setup());
    }

    fn unlock_if_held(&mut self, owner: MutexOwner, assertions: &mut SmokeAssertions) {
        if self.lock.locked() && self.lock.owner() == owner {
            assertions.assert_ok("teardown unlock mutex", self.lock.unlock_owner(owner));
        }
    }
}

struct SetupReadyScenario {
    fixture: MutexFixture,
}

impl SetupReadyScenario {
    const fn new() -> Self {
        Self {
            fixture: MutexFixture::new(),
        }
    }
}

impl SmokeScenario for SetupReadyScenario {
    fn name(&self) -> &'static str {
        "mutex.setup_ready"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert("mutex ready", self.fixture.lock.state() == State::Ready);
        assertions.assert(
            "mutex static init",
            self.fixture.lock.init_kind() == MutexInitKind::StaticInitializer,
        );
        assertions.assert("storage bound", self.fixture.lock.storage_bound());
        assertions.assert("wait queue ready", self.fixture.lock.wait_queue_ready());
        assertions.assert(
            "wait lock deferred",
            self.fixture.lock.wait_lock_internal_deferred(),
        );
        assertions.assert("mutex initially unlocked", self.fixture.lock.unlocked());
        assertions.assert("no owner", self.fixture.lock.owner() == MutexOwner::None);
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_if_held(MutexOwner::KernelInitTask, assertions);
    }
}

struct UnpreparedOperationsScenario {
    fixture: MutexFixture,
}

impl UnpreparedOperationsScenario {
    const fn new() -> Self {
        Self {
            fixture: MutexFixture::new(),
        }
    }
}

impl SmokeScenario for UnpreparedOperationsScenario {
    fn name(&self) -> &'static str {
        "mutex.unprepared_operations"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert_fail(
            "lock before setup",
            self.fixture.lock.lock_owner(MutexOwner::KernelInitTask),
        );
        assertions.assert_fail(
            "unlock before setup",
            self.fixture.lock.unlock_owner(MutexOwner::KernelInitTask),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

struct LockUnlockScenario {
    fixture: MutexFixture,
}

impl LockUnlockScenario {
    const fn new() -> Self {
        Self {
            fixture: MutexFixture::new(),
        }
    }
}

impl SmokeScenario for LockUnlockScenario {
    fn name(&self) -> &'static str {
        "mutex.lock_unlock"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "lock acquired",
            self.fixture.lock.lock_owner(MutexOwner::KernelInitTask)
                == Ok(MutexLockOutcome::Acquired),
        );
        assertions.assert("lock held", self.fixture.lock.locked());
        assertions.assert(
            "owner set",
            self.fixture.lock.owner() == MutexOwner::KernelInitTask,
        );
        assertions.assert_fail(
            "wrong owner unlock",
            self.fixture.lock.unlock_owner(MutexOwner::SmokeMutexTask),
        );
        assertions.assert_ok(
            "owner unlock",
            self.fixture.lock.unlock_owner(MutexOwner::KernelInitTask),
        );
        assertions.assert("lock released", self.fixture.lock.unlocked());
        assertions.assert("lock count", self.fixture.lock.lock_entered_count() == 1);
        assertions.assert("unlock count", self.fixture.lock.unlock_exited_count() == 1);
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_if_held(MutexOwner::KernelInitTask, assertions);
    }
}

struct ContendedWakeScenario {
    fixture: MutexFixture,
}

impl ContendedWakeScenario {
    const fn new() -> Self {
        Self {
            fixture: MutexFixture::new(),
        }
    }
}

impl SmokeScenario for ContendedWakeScenario {
    fn name(&self) -> &'static str {
        "mutex.contended_wake"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture.setup_ready(assertions);
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        assertions.assert(
            "first owner acquired",
            self.fixture.lock.lock_owner(MutexOwner::KernelInitTask)
                == Ok(MutexLockOutcome::Acquired),
        );
        assertions.assert(
            "second owner blocked",
            self.fixture.lock.lock_owner(MutexOwner::SmokeMutexTask)
                == Ok(MutexLockOutcome::Blocked),
        );
        assertions.assert("contended count", self.fixture.lock.contended_count() == 1);
        assertions.assert_ok(
            "first owner unlock",
            self.fixture.lock.unlock_owner(MutexOwner::KernelInitTask),
        );
        assertions.assert("wake count", self.fixture.lock.wake_count() == 1);
        assertions.assert(
            "second owner acquired after wake",
            self.fixture.lock.lock_owner(MutexOwner::SmokeMutexTask)
                == Ok(MutexLockOutcome::Acquired),
        );
        assertions.assert_ok(
            "second owner unlock",
            self.fixture.lock.unlock_owner(MutexOwner::SmokeMutexTask),
        );
        assertions.assert("final unlocked", self.fixture.lock.unlocked());
        assertions.assert("lock count", self.fixture.lock.lock_entered_count() == 2);
        assertions.assert("unlock count", self.fixture.lock.unlock_exited_count() == 2);
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        self.fixture
            .unlock_if_held(MutexOwner::KernelInitTask, assertions);
        self.fixture
            .unlock_if_held(MutexOwner::SmokeMutexTask, assertions);
    }
}

struct CooperativeContentionScenario;

impl CooperativeContentionScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for CooperativeContentionScenario {
    fn name(&self) -> &'static str {
        "mutex.cooperative_contention"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        shared.reset();
        assertions.assert_ok("preset shared mutex", shared.lock.preset_static());
        assertions.assert_ok("setup shared mutex", shared.lock.setup());

        let ctx = context();
        assertions.assert("scheduler online", ctx.scheduler.state() == State::Online);
        assertions.assert(
            "current kernel init",
            ctx.boot_cpu_current_task.current_is_kernel_init(),
        );
        assertions.assert_ok(
            "setup smoke mutex task",
            ctx.setup_smoke_mutex_task(smoke_mutex_task_entry),
        );
        assertions.assert_ok("enqueue smoke mutex task", ctx.enqueue_smoke_mutex_task());
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        {
            let shared = shared_state();
            assertions.assert(
                "kernel init acquired",
                shared.lock.lock_owner(MutexOwner::KernelInitTask)
                    == Ok(MutexLockOutcome::Acquired),
            );
        }

        assertions.assert_ok("schedule contending task", context().schedule_current());

        {
            let shared = shared_state();
            assertions.assert("task entry ran", shared.task_entry_ran);
            assertions.assert("task blocked on mutex", shared.task_blocked);
            assertions.assert("task yielded after block", shared.task_yielded_after_block);
            assertions.assert("contended once", shared.lock.contended_count() == 1);
            assertions.assert(
                "current returned kernel init",
                context().boot_cpu_current_task.current() == CurrentTaskRef::KernelInit,
            );
            assertions.assert_ok(
                "kernel init unlock",
                shared.lock.unlock_owner(MutexOwner::KernelInitTask),
            );
            assertions.assert("wake recorded", shared.lock.wake_count() == 1);
        }

        let mut attempts = 0usize;
        while attempts < SCHEDULE_ATTEMPTS {
            if shared_state().task_completed {
                break;
            }
            assertions.assert_ok("schedule awakened task", context().schedule_current());
            attempts += 1;
        }

        let shared = shared_state();
        assertions.assert("bounded completion", attempts < SCHEDULE_ATTEMPTS);
        assertions.assert("task retry acquired", shared.task_retry_acquired);
        assertions.assert("task unlocked", shared.task_unlocked);
        assertions.assert("task completed", shared.task_completed);
        assertions.assert("final unlocked", shared.lock.unlocked());
        assertions.assert("lock count", shared.lock.lock_entered_count() == 2);
        assertions.assert("unlock count", shared.lock.unlock_exited_count() == 2);
        assertions.assert(
            "smoke mutex task yielded",
            context().scheduler.smoke_mutex_task().yielded_back(),
        );
        assertions.assert(
            "smoke mutex task dequeued",
            !context()
                .scheduler
                .boot_runqueue()
                .contains_task(context().scheduler.smoke_mutex_task().task_id()),
        );
    }

    fn teardown(&mut self, assertions: &mut SmokeAssertions) {
        let shared = shared_state();
        if shared.lock.locked() && shared.lock.owner() == MutexOwner::KernelInitTask {
            assertions.assert_ok(
                "teardown unlock kernel init",
                shared.lock.unlock_owner(MutexOwner::KernelInitTask),
            );
        }
        if shared.lock.locked() && shared.lock.owner() == MutexOwner::SmokeMutexTask {
            assertions.assert_ok(
                "teardown unlock smoke mutex",
                shared.lock.unlock_owner(MutexOwner::SmokeMutexTask),
            );
        }
    }
}

struct MutexSmokeShared {
    lock: Mutex,
    task_entry_ran: bool,
    task_blocked: bool,
    task_yielded_after_block: bool,
    task_retry_acquired: bool,
    task_unlocked: bool,
    task_completed: bool,
}

impl MutexSmokeShared {
    const fn new() -> Self {
        Self {
            lock: Mutex::new_static(),
            task_entry_ran: false,
            task_blocked: false,
            task_yielded_after_block: false,
            task_retry_acquired: false,
            task_unlocked: false,
            task_completed: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

static mut MUTEX_SMOKE_SHARED: MutexSmokeShared = MutexSmokeShared::new();

fn shared_state() -> &'static mut MutexSmokeShared {
    // SAFETY: smoke runs on the boot CPU with cooperative task switches. This
    // state models the shared object observed by the two smoke tasks.
    unsafe { &mut *core::ptr::addr_of_mut!(MUTEX_SMOKE_SHARED) }
}

extern "C" fn smoke_mutex_task_entry() -> ! {
    if context().mark_smoke_mutex_entry_ran().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_entry_ran = true;

    match shared_state().lock.lock_owner(MutexOwner::SmokeMutexTask) {
        Ok(MutexLockOutcome::Blocked) => {
            shared_state().task_blocked = true;
        }
        Ok(MutexLockOutcome::Acquired) | Err(_) => crate::arch::riscv64::sbi::system_shutdown(),
    }

    shared_state().task_yielded_after_block = true;
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    match shared_state().lock.lock_owner(MutexOwner::SmokeMutexTask) {
        Ok(MutexLockOutcome::Acquired) => {
            shared_state().task_retry_acquired = true;
        }
        Ok(MutexLockOutcome::Blocked) | Err(_) => crate::arch::riscv64::sbi::system_shutdown(),
    }
    if shared_state()
        .lock
        .unlock_owner(MutexOwner::SmokeMutexTask)
        .is_err()
    {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    shared_state().task_unlocked = true;
    shared_state().task_completed = true;

    if context().mark_smoke_mutex_yielded_back().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().dequeue_smoke_mutex_task().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }
    if context().schedule_current().is_err() {
        crate::arch::riscv64::sbi::system_shutdown();
    }

    loop {
        core::hint::spin_loop();
    }
}
