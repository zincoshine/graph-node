#[macro_use]
extern crate slog;
extern crate thegraph;

mod adapter;
mod server;

pub use self::adapter::RuntimeAdapter;
