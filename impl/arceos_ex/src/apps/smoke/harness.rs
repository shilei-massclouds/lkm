use crate::objects::printk;

use super::SmokeResult;

pub struct SmokeAssertions {
    scenario: &'static str,
    failed: bool,
}

impl SmokeAssertions {
    pub const fn new(scenario: &'static str) -> Self {
        Self {
            scenario,
            failed: false,
        }
    }

    pub const fn failed(&self) -> bool {
        self.failed
    }

    pub const fn result(&self) -> SmokeResult {
        if self.failed {
            SmokeResult::Failed
        } else {
            SmokeResult::Passed
        }
    }

    pub fn assert(&mut self, label: &'static str, condition: bool) {
        if !condition {
            self.record_failure(label);
        }
    }

    pub fn assert_ok<T, E>(&mut self, label: &'static str, result: Result<T, E>) {
        self.assert(label, result.is_ok());
    }

    pub fn assert_fail<T, E>(&mut self, label: &'static str, result: Result<T, E>) {
        self.assert(label, result.is_err());
    }

    #[allow(dead_code)]
    pub fn assert_timeout<F>(&mut self, label: &'static str, attempts: usize, mut condition: F)
    where
        F: FnMut() -> bool,
    {
        let mut remaining = attempts;
        while remaining != 0 {
            if condition() {
                return;
            }
            remaining -= 1;
        }
        self.record_failure(label);
    }

    fn record_failure(&mut self, label: &'static str) {
        self.failed = true;
        printk::write_str("    assert failed: ");
        printk::write_str(self.scenario);
        printk::write_str("::");
        printk::write_str(label);
        printk::write_str("\n");
    }
}

pub trait SmokeScenario {
    fn name(&self) -> &'static str;
    fn setup(&mut self, assertions: &mut SmokeAssertions);
    fn run(&mut self, assertions: &mut SmokeAssertions);
    fn teardown(&mut self, assertions: &mut SmokeAssertions);
}

pub struct SmokeSuite {
    failed: bool,
}

impl SmokeSuite {
    pub const fn new() -> Self {
        Self { failed: false }
    }

    pub fn scenario<S: SmokeScenario>(&mut self, scenario: &mut S) {
        if run_scenario(scenario).is_failed() {
            self.failed = true;
        }
    }

    pub const fn result(&self) -> SmokeResult {
        if self.failed {
            SmokeResult::Failed
        } else {
            SmokeResult::Passed
        }
    }
}

pub fn run_scenario<S: SmokeScenario>(scenario: &mut S) -> SmokeResult {
    printk::write_str("  scenario ");
    printk::write_str(scenario.name());
    printk::write_str("\n");

    let mut assertions = SmokeAssertions::new(scenario.name());
    scenario.setup(&mut assertions);
    if !assertions.failed() {
        scenario.run(&mut assertions);
    }
    scenario.teardown(&mut assertions);

    match assertions.result() {
        SmokeResult::Passed => {
            printk::write_str("    ok\n");
            SmokeResult::Passed
        }
        SmokeResult::Failed => {
            printk::write_str("    FAILED\n");
            SmokeResult::Failed
        }
    }
}
