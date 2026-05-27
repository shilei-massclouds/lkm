pub mod hello;

#[cfg(app_hello)]
pub use hello::run;

#[cfg(not(app_hello))]
compile_error!("unsupported APP selection; build with APP=hello");
