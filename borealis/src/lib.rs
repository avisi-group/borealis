#![warn(missing_docs)]

//! Sail frontend for GenSim

use {
    crate::{
        genc::{Description, Instruction},
        instruction::{get_instructions, process_instruction},
    },
    color_eyre::{eyre::eyre, Result},
    errctx::PathCtx,
    lz4_flex::frame::FrameDecoder as Lz4Decoder,
    sail::{ast::Ast, runtime::DEFAULT_RUNTIME_THREAD_STACK_SIZE},
    std::{io, path::PathBuf, thread},
};

pub mod genc;
pub mod instruction;

/// Borealis error
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum Error {
    /// IO error
    Io(#[from] PathCtx<io::Error>),
    /// Error from Sail compiler
    Sail(#[from] sail::error::Error),
    /// GenC export directory {0:?} not found
    OutDirectoryNotFound(PathBuf),
    /// GenC export directory {0:?} not empty
    OutDirectoryNotEmpty(PathBuf),
}

/// Compiles a Sail ISA specification to a GenC description
pub fn sail_to_genc(ast: &Ast, _jib: &str) -> Description {
    let instructions = get_instructions(ast);

    let mut description = Description::empty();

    description.instructions = instructions
        .into_iter()
        .map(|clause| process_instruction(ast, &clause))
        .map(|(name, format, execute)| (name.to_string(), Instruction { format, execute }))
        .collect();

    description
}

/// Deserializes an AST from a compressed bincode reader.
///
/// Internally, deserialization is performed on a new thread with a sufficient stack size to perform the deserialization.
pub fn deserialize_compressed_ast<R: io::Read + Send + 'static>(
    reader: R,
) -> Result<(Ast, String)> {
    let thread = thread::Builder::new().stack_size(DEFAULT_RUNTIME_THREAD_STACK_SIZE);

    let handle = thread
        .spawn(move || bincode::deserialize_from::<_, (Ast, String)>(Lz4Decoder::new(reader)))?;

    let out = handle
        .join()
        .map_err(|_| eyre!("Failed to join aassociated thread"))??;

    Ok(out)
}
