#[macro_export]
macro_rules! define_initcall {
    ($level:expr, $section:literal, $entry:ident) => {
        const _: () = {
            #[used]
            #[unsafe(link_section = $section)]
            static INITCALL_ENTRY: $crate::objects::initcall::InitcallEntry =
                $crate::objects::initcall::InitcallEntry::new($level, stringify!($entry), $entry);
        };
    };
}

#[macro_export]
macro_rules! pure_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Pure,
            ".initcall.pure",
            $entry
        );
    };
}

#[macro_export]
macro_rules! core_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Core,
            ".initcall.core",
            $entry
        );
    };
}

#[macro_export]
macro_rules! postcore_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Postcore,
            ".initcall.postcore",
            $entry
        );
    };
}

#[macro_export]
macro_rules! arch_initcall_sync {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Arch,
            ".initcall.arch",
            $entry
        );
    };
}

#[macro_export]
macro_rules! subsys_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Subsys,
            ".initcall.subsys",
            $entry
        );
    };
}

#[macro_export]
macro_rules! fs_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Fs,
            ".initcall.fs",
            $entry
        );
    };
}

#[macro_export]
macro_rules! device_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Device,
            ".initcall.device",
            $entry
        );
    };
}

#[macro_export]
macro_rules! late_initcall {
    ($entry:ident) => {
        $crate::define_initcall!(
            $crate::objects::initcall::InitcallLevelName::Late,
            ".initcall.late",
            $entry
        );
    };
}
