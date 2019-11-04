#![deny(warnings, rust_2018_idioms)]

pub mod common;
pub mod introspection;
pub mod migration_engine;

use serde::Serialize;

pub trait UserFacingError: serde::Serialize {
    const ERROR_CODE: &'static str;

    fn message(&self) -> String;
}

#[derive(Serialize)]
pub struct KnownError {
    message: String,
    meta: serde_json::Value,
    error_code: &'static str,
}

impl KnownError {
    pub fn new<T: UserFacingError>(inner: T) -> Result<KnownError, serde_json::Error> {
        Ok(KnownError {
            message: inner.message(),
            meta: serde_json::to_value(&inner)?,
            error_code: T::ERROR_CODE,
        })
    }
}

#[derive(Serialize)]
pub struct UnknownError {
    pub message: String,
    pub backtrace: Option<String>,
}
