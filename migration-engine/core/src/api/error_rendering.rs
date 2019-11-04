use crate::error::Error as CrateError;
use jsonrpc_core::types::Error as JsonRpcError;
use user_facing_errors::{migration_engine::*, Error, KnownError, UnknownError};

pub(super) fn render_error(crate_error: CrateError) -> Error {
    match crate_error {
        _ => (),
    };

    UnknownError {
        message: format!("{}", crate_error),
        backtrace: None,
    }
}

pub(super) fn render_jsonrpc_error(crate_error: CrateError) -> JsonRpcError {
    let prisma_error = render_error(crate_error);

    let error_rendering_result: Result<_, _> = match prisma_error {
        user_facing_errors::Error::Known(known) => serde_json::to_value(prisma_error).map(|data| {
            JsonRpcError {
                // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
                // prisma error code, which is located in `data`.
                code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
                message: "An error happened. Check the data field for details.".to_string(),
                data: Some(data),
            }
        }),
    };

    match error_rendering_result {
        Ok(err) => err,
        Err(_) => unimplemented!("error handling in error rendering"),
    }

    // match error {
    //     crate::error::Error::CommandError(command_error) => {
    //         let json = serde_json::to_value(command_error).unwrap();

    //         JsonRpcError {
    //             code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
    //             message: "An error happened. Check the data field for details.".to_string(),
    //             data: Some(json),
    //         }
    //     }
    //     crate::error::Error::BlockingError(_) => JsonRpcError {
    //         code: jsonrpc_core::types::error::ErrorCode::ServerError(4467),
    //         message: "The RPC threadpool is exhausted. Add more worker threads.".to_string(),
    //         data: None,
    //     },
    //     err => panic!(
    //         "An unexpected error happened. Maybe we should build a handler for these kind of errors? {:?}",
    //         err
    //     ),
    // }
}
