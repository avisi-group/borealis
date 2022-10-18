//! Error handling for Sail interface

use {
    crate::runtime::Request,
    fragile::Fragile,
    std::{
        fmt::Debug,
        sync::mpsc::{RecvError, SendError},
    },
};

/// Sail error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum Error {
    /// Error returned by OCaml function: {0:?}
    OcamlFunction(
        // fragile to make `ocaml::Error` `Send` (may contain a raw pointer but since we are not
        // dereferencing it is safe to send across threads)
        Fragile<ocaml::Error>,
    ),

    /// Error when communicating with runtime worker thread: {0}
    RuntimeCommunication(#[from] ChannelError),
}

impl From<ocaml::Error> for Error {
    fn from(e: ocaml::Error) -> Self {
        Self::OcamlFunction(Fragile::new(e))
    }
}

/// MPSC channel error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum ChannelError {
    /// Sending failed: {0}
    Send(#[from] SendError<Request>),
    /// Receiving failed: {0}
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
