//! Error handling for Sail interface

use {
    crate::{
        ast::L,
        runtime::Request,
        types::{OCamlString, Position},
    },
    ocaml::{CamlError, FromValue, Int},
    std::{
        fmt::Debug,
        sync::mpsc::{RecvError, SendError},
    },
};

/// Main `sail` crate error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum Error {
    /// OCaml function error
    OCamlFunction(#[from] OCamlError),

    /// Error when communicating with runtime worker thread
    RuntimeCommunication(#[from] ChannelError),

    /// Sail wrapper function error
    Wrapper(#[from] WrapperError),

    /// Sail JSON loading error
    ModelConfig(#[from] crate::json::Error),
}

impl From<ocaml::Error> for Error {
    fn from(e: ocaml::Error) -> Self {
        OCamlError::from(e).into()
    }
}

/// Thread-safe version of ocaml::Error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum OCamlError {
    /// A value cannot be called using callback functions
    NotCallable,

    /// Array is not a double array
    NotDoubleArray,

    /// Error message
    Message(&'static str),

    /// General error: {0}
    Error(String),

    /// OCaml exceptions
    Caml(#[from] OCamlErrorInner),
}

impl From<ocaml::Error> for OCamlError {
    fn from(e: ocaml::Error) -> Self {
        match e {
            ocaml::Error::NotCallable => Self::NotCallable,
            ocaml::Error::NotDoubleArray => Self::NotDoubleArray,
            ocaml::Error::Message(m) => Self::Message(m),
            ocaml::Error::Error(e) => Self::Error(e.to_string()),
            ocaml::Error::Caml(e) => Self::Caml((&e).into()),
        }
    }
}

/// Thread-safe version of ocaml::CamlError
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum OCamlErrorInner {
    /// Not_found
    NotFound,

    /// Failure: {0}
    Failure(&'static str),

    /// Invalid_argument: {0}
    InvalidArgument(&'static str),

    /// Out_of_memory
    OutOfMemory,

    /// Stack_overflow
    StackOverflow,

    /// Sys_error: {0}
    SysError(String),

    /// End_of_file
    EndOfFile,

    /// Zero_divide
    ZeroDivide,

    /// Array bound error
    ArrayBoundError,

    /// Sys_blocked_io
    SysBlockedIo,

    /// A pre-allocated OCaml exception: {0}
    Exception(String),

    /// An exception type and argument: ({0}, {1})
    WithArg(String, String),
}

impl From<&ocaml::CamlError> for OCamlErrorInner {
    fn from(e: &ocaml::CamlError) -> Self {
        match e {
            CamlError::NotFound => Self::NotFound,
            CamlError::Failure(s) => Self::Failure(s),
            CamlError::InvalidArgument(s) => Self::InvalidArgument(s),
            CamlError::OutOfMemory => Self::OutOfMemory,
            CamlError::StackOverflow => Self::StackOverflow,
            CamlError::SysError(v) => Self::SysError(format!("{:?}", v)),
            CamlError::EndOfFile => Self::EndOfFile,
            CamlError::ZeroDivide => Self::ZeroDivide,
            CamlError::ArrayBoundError => Self::ArrayBoundError,
            CamlError::SysBlockedIo => Self::SysBlockedIo,
            CamlError::Exception(v) => {
                // unsafe if `from` is called outside of the worker thread
                let msg = unsafe { v.exception_to_string() }.unwrap_or_else(|e| {
                    format!(
                        "Failed to convert exception to string due to UTF-8 error: {}",
                        e
                    )
                });

                Self::Exception(msg)
            }
            CamlError::WithArg(typ, arg) => {
                Self::WithArg(format!("{:?}", typ), format!("{:?}", arg))
            }
        }
    }
}

/// MPSC channel error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum ChannelError {
    /// Sending failed
    Send(#[from] SendError<Request>),
    /// Receiving failed
    Receive(#[from] RecvError),
}

impl From<SendError<Request>> for Error {
    fn from(e: SendError<Request>) -> Self {
        Error::RuntimeCommunication(e.into())
    }
}

impl From<RecvError> for Error {
    fn from(e: RecvError) -> Self {
        Error::RuntimeCommunication(e.into())
    }
}

/// Error from OCaml wrapper function around Sail library functions
#[derive(Debug, displaydoc::Display, thiserror::Error, FromValue)]
pub enum WrapperError {
    /// Exception occurred: "{message}" with backtrace: "{backtrace}"
    Exception {
        /// Exception message
        message: String,
        /// Recorded backtrace
        backtrace: String,
    },
    /// Error from Sail compiler
    Sail(#[from] SailCompilerError),
}

/// Sail compiler error
#[derive(Debug, displaydoc::Display, thiserror::Error, FromValue)]
pub enum SailCompilerError {
    /// General error: {1:?} @ {0:?}
    General(L, OCamlString),
    /// Unreachable error in {1:?}: {3:?} @ {0:?}
    Unreachable(L, (OCamlString, Int, Int, Int, Int), (), OCamlString),
    /// Todo error: {1:?} @ {0:?}
    Todo(L, OCamlString),
    /// Syntax error: {1:?} @ {0:?}
    Syntax(Position, OCamlString),
    /// Syntax location error: {1:?} @ {0:?}
    SyntaxLocation(L, OCamlString),
    /// Lexical error: {1:?} @ {0:?}
    Lexical(Position, OCamlString),
    /// Type error: {1:?} @ {0:?}
    Type(L, OCamlString),
}
