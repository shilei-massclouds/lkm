#[cfg(app_hello)]
pub mod hello;
#[cfg(app_smoke)]
pub mod smoke;
#[cfg(app_user_hello)]
pub mod user_hello;

#[cfg(app_smoke)]
pub use smoke::run;

#[cfg(app_user_hello)]
pub use user_hello::run;

#[cfg(app_hello)]
pub use hello::run;

#[cfg(not(any(app_smoke, app_hello, app_user_hello)))]
compile_error!("unsupported APP selection; build with APP=smoke, APP=hello or APP=user-hello");
