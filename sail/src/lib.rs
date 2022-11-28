#![warn(missing_docs)]

//! Rust interface to `Sail` compiler library

use {
    crate::{
        ast::Ast, error::Error, json::ModelConfig, runtime::Runtime, type_check::Env,
        types::OCamlString,
    },
    once_cell::sync::Lazy,
    parking_lot::Mutex,
    std::path::Path,
};

pub mod ast;
pub mod error;
pub mod json;
pub mod num;
mod runtime;
pub mod type_check;
pub mod types;

/// Global runtime shared by all public functions
static RT: Lazy<Mutex<Runtime>> = Lazy::new(|| Mutex::new(Runtime::new()));

/// Parses supplied Sail files and returns the AST
///
/// From Sail internal docs:
///
/// This function parses all the files passed to Sail, and then concatenates
/// their ASTs. The pre-processor is then run, which evaluates `$directive`
/// statements in Sail, such as
///
/// ```sail
/// $include <prelude.sail>
/// ```
///
/// Unlike the C pre-processor the Sail pre-processor operates over actual
/// Sail ASTs rather than strings. This can recursively include other
/// files into the AST, as well as add/remove parts of the AST with
/// `$ifdef` etc. Directives that are used are preserved in the AST, so
/// they also function as a useful way to pass auxiliary information to
/// the various Sail backends.
///
/// The initial check mentioned above is then run to desugar the AST, and
/// then the type-checker is run which produces a fully type-checked
/// AST. Type annotations are attached to every node (for which an
/// annotation makes sense) using the aux constructors.
///
/// After type-checking the Sail scattered definitions are de-scattered
/// into single functions.
pub fn load_files(config: ModelConfig) -> Result<(OCamlString, Ast, Env), Error> {
    Ok(RT.lock().load_files(config)?)
}

/// Loads Sail files from `sail.json` model configuration.
pub fn load_from_config<P: AsRef<Path>>(config_path: P) -> Result<(OCamlString, Ast, Env), Error> {
    let config = dbg!(json::ModelConfig::load(config_path.as_ref())?);

    load_files(config)
}

#[cfg(test)]
mod tests {
    use {
        crate::{json::ModelConfig, load_files, load_from_config, RT},
        proptest::{bits, collection::vec, prelude::*},
    };

    proptest! {
        /// Checks equivalence between libsail dedup function and Rust stdlib dedup.
        ///
        /// Used as smoke test that OCaml interop is functioning correctly (intentionally doing a lot of allocating, many function calls, etc).
        #[test]
        fn smoke_test(v in vec(bits::i32::ANY, 0..1000)) {
            let mut v_d = v.clone();
            v_d.sort();

            v_d.dedup();

            let mut out = RT.lock().dedup(v).unwrap();
            out.sort();
            assert_eq!(out, v_d);
        }
    }

    #[test]
    fn load_files_empty() {
        insta::assert_json_snapshot!(load_files(ModelConfig {
            files: vec![],
            options: Default::default()
        })
        .unwrap());
    }

    #[test]
    fn load_from_config_arm() {
        let res = load_from_config("../testdata/sail-arm.json").unwrap();

        insta::with_settings!({filters => vec![
            (r#""kind_identifier": \{[\s]*"String":.*[\s]*\}"#, r#""kind_identifier": {}"#),
            (r#""kind_identifier": \{\s*"Vec": \[[\s,0-9]*\]\s*\}"#, r#""kind_identifier": {}"#),
        ]}, {
            insta::assert_json_snapshot!(res);
        });
    }
}
