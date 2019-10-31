#![deny(warnings, rust_2018_idioms)]

pub mod common;
pub mod introspection;
pub mod migration_engine;

pub trait UserFacingError {
    const ERROR_CODE: &'static str;

    fn message(&self) -> String;
}
