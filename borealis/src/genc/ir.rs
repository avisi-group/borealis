//! Intermediate representation of GenC files
//!
//! This means we can do two things:
//!
//! 1. Maintain a clear, understandable user-facing structure for a GenC description (`crate::genc::Description`)
//! 2. Use `std::fmt::Display` and recursion to generate each file (a nice pattern ), without requiring externally maintained state

use {
    crate::genc::{
        Bank, Endianness, RegisterSpace, Typ, BEHAVIOURS_FILENAME, EXECUTE_FILENAME, ISA_FILENAME,
    },
    std::fmt::{self, Display},
};

/// GenC files which may be rendered using only the `std::fmt::Display` trait, achieved by duplicating several pieces of information between structs.
///
/// Only for internal use; users of library should use `crate::genc::Description` which does not contain duplicated information.
#[derive(Debug, Clone)]
pub struct Files {
    pub main: Main,
    pub isa: Isa,
    pub execute: Execute,
    pub behaviours: Behaviours,
}

impl Display for Endianness {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Endianness::BigEndian => write!(f, "big"),
            Endianness::LittleEndian => write!(f, "little"),
        }
    }
}

/// Main GenC file
#[derive(Debug, Clone)]
pub struct Main {
    pub name: String,

    /// Endianness of the architecture
    pub endianness: Endianness,

    /// Word size in bytes
    pub wordsize: u32,

    /// Register definitions
    pub registers: Vec<RegisterSpace>,
}

impl Display for Main {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_header(f)?;
        writeln!(f, "AC_ARCH({})", self.name)?;
        writeln!(f, "{{")?;

        writeln!(
            f,
            "\tac_mem Data({}, {}, {}, {});",
            self.wordsize, self.wordsize, self.endianness, 0
        )?;
        writeln!(
            f,
            "\tac_mem Fetch({}, {}, {}, {});",
            self.wordsize, self.wordsize, self.endianness, 1
        )?;
        writeln!(f, "")?;
        writeln!(f, "\tac_wordsize {};", self.wordsize)?;
        writeln!(f, "")?;

        for register in &self.registers {
            write!(f, "{}", register)?;
        }

        writeln!(f, "")?;

        writeln!(f, "\tARCH_CTOR(arm) {{")?;
        writeln!(f, "\t\tac_isa(\"{}\");", ISA_FILENAME)?;
        writeln!(f, "\t\tset_endian(\"{}\");", self.endianness)?;
        writeln!(f, "\t}};")?;

        writeln!(f, "}};")?;

        Ok(())
    }
}

impl Display for RegisterSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "\tac_regspace({}) {{", self.size)?;
        for bank in &self.banks {
            write!(f, "{}", bank)?;
        }
        writeln!(f, "\t}}")
    }
}

impl Display for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(
            f,
            "\t\tbank {} ({}, {}, {}, {}, {}, {}, {});",
            self.name,
            self.typ,
            self.offset,
            self.count,
            self.stride,
            self.element_count,
            self.element_size,
            self.element_stride
        )
    }
}

impl Display for Typ {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Uint8 => "uint8",
                Self::Uint16 => "uint16",
                Self::Uint32 => "uint32",
                Self::Uint64 => "uint64",
                Self::Uint128 => "uint128",
                Self::Float => "float",
                Self::Double => "double",
            }
        )
    }
}

/// ISA GenC file
#[derive(Debug, Clone)]
pub struct Isa {
    pub name: String,

    pub predicated: bool,

    /// Instruction size in bytes
    pub fetchsize: u32,

    /// Instruction decode format strings
    pub formats: Vec<Format>,
}

impl Display for Isa {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_header(f)?;
        writeln!(f, "AC_ISA({})", self.name)?;
        writeln!(f, "{{")?;

        writeln!(f, "\tac_fetchsize {};", self.fetchsize)?;
        writeln!(
            f,
            "\tac_predicated \"{}\";",
            match self.predicated {
                true => "yes",
                false => "no",
            }
        )?;

        for format in &self.formats {
            write!(f, "{}", format)?;
        }

        writeln!(f, "")?;
        writeln!(f, "\tISA_CTOR(arm) {{")?;
        writeln!(f, "\t\tac_behaviours(\"{}\");", BEHAVIOURS_FILENAME)?;
        writeln!(f, "\t\tac_execute(\"{}\");", EXECUTE_FILENAME)?;
        writeln!(f, "\t}};")?;

        writeln!(f, "}}")?;

        Ok(())
    }
}

/// Format string describing the en/decoding of an instruction
#[derive(Debug, Clone)]
pub struct Format {
    /// Format identifier
    pub format_ident: String,
    /// Instruction identifier
    pub instruction_ident: String,
    /// Format string
    pub contents: String,
}

impl Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "\tac_format {} = \"{}\";",
            self.format_ident, self.contents
        )?;
        writeln!(
            f,
            "\tac_instruction<{}> {};",
            self.format_ident, self.instruction_ident
        )?;

        Ok(())
    }
}

/// Execution GenC file containing instruction implementations
#[derive(Debug, Clone)]
pub struct Execute(pub Vec<Function>);

impl Display for Execute {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_header(f)?;
        for func in &self.0 {
            writeln!(f, "{}", func)?;
        }
        Ok(())
    }
}

/// Behaviours GenC file
#[derive(Debug, Clone)]
pub struct Behaviours(pub Vec<Function>);

impl Display for Behaviours {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write_header(f)?;
        for func in &self.0 {
            writeln!(f, "{}", func)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub kind: FunctionKind,
    pub name: String,
    pub body: String,
}

impl Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}({}) {{", self.kind, self.name)?;
        writeln!(f, "{}", self.body)?;
        writeln!(f, "}}")
    }
}

#[derive(Debug, Clone)]
pub enum FunctionKind {
    Behaviour,
    Execute,
}

impl Display for FunctionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Behaviour => "behaviour",
                Self::Execute => "execute",
            }
        )
    }
}

/// Creates a file and writes the initial comment
fn write_header(f: &mut std::fmt::Formatter) -> fmt::Result {
    writeln!(f, "/* GENERATED BY BOREALIS */\n\n")
}
