#[cfg(app_hello)]
pub mod hello;
#[cfg(any(app_smoke, checkpoint_handler_smoke))]
pub mod smoke;

#[cfg(app_smoke)]
pub use smoke::run;

#[cfg(app_hello)]
pub use hello::run;

#[cfg(not(any(app_smoke, app_hello)))]
compile_error!("unsupported APP selection; build with APP=smoke or APP=hello");
