//! Error handling for Sail interface

use {
    crate::runtime::Request,
    ocaml::{CamlError, FromValue, Value},
    std::{
        fmt::Debug,
        sync::mpsc::{RecvError, SendError},
    },
};

/// Sail error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum Error {
    /// Error returned by OCaml function
    OCamlFunction(ocaml::Error),

    /// Error when communicating with runtime worker thread
    RuntimeCommunication(#[from] ChannelError),

    /// Error returned by Sail wrapper function
    Wrapper(#[from] WrapperError),
}

// required as ocaml::Error can contain raw pointers
unsafe impl Send for Error {}
unsafe impl Sync for Error {}

impl From<ocaml::Error> for Error {
    fn from(e: ocaml::Error) -> Self {
        if let ocaml::Error::Caml(CamlError::Exception(val)) = &e {
            // unsafe if `from` is called outside of the worker thread
            let msg = unsafe { val.exception_to_string() }.unwrap_or_else(|e| {
                format!(
                    "Failed to convert exception to string due to UTF-8 error: {}",
                    e
                )
            });

            Self::OCamlFunction(ocaml::Error::Error(msg.into()))
        } else {
            Self::OCamlFunction(e)
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
    /// Sail error: {0:?}
    Sail(SailError),
}

/// Sail library error
#[derive(Debug, displaydoc::Display, thiserror::Error, FromValue)]
pub enum SailError {
    /// General
    General(Value, String),
    /// Unreachable
    Unreachable(
        Value,
        (String, ocaml::Int, ocaml::Int, ocaml::Int, ocaml::Int),
        Value,
        String,
    ),
    /// Todo
    Todo(Value, String),
    /// Syntax
    Syntax(Value, String),
    /// Syntax location
    SyntaxLocation(Value, String),
    /// Lexer
    Lexer(Value, String),
    /// Type
    Type(Value, String),
}
