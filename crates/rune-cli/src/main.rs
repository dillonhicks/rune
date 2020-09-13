//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! A cli for the [Rune Language].
//!
//! If you're in the repo, you can take it for a spin with:
//!
//! ```text
//! cargo run -- scripts/hello_world.rn
//! ```
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//! [runestick]: https://github.com/rune-rs/rune

use anyhow::Result;
use rune::termcolor::{ColorChoice, StandardStream};
<<<<<<< HEAD
use std::env;
use std::path:: PathBuf;
use rune_interpreter::{Interpreter, Config, InteractiveInterpreter};
=======
use rune::EmitDiagnostics as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use structopt::StructOpt;


#[derive(Default, Debug, Clone, StructOpt)]
#[structopt(name = "rune", about = "The Rune Language")]
struct Args {
    /// Run the interpreter in interactive mode (REPL).
    #[structopt(short, long)]
    interactive: bool,
    /// Provide detailed tracing for each instruction executed.
    #[structopt(short, long)]
    trace: bool,
    /// Dump everything.
    #[structopt(short, long)]
    dump: bool,
    /// Dump default information about unit.
    #[structopt(long)]
    dump_unit: bool,
    /// Dump unit instructions.
    #[structopt(long)]
    dump_instructions: bool,
    /// Dump the state of the stack after completion.
    ///
    /// If compiled with `--trace` will dump it after each instruction.
    #[structopt(long)]
    dump_stack: bool,
    /// Dump dynamic functions.
    #[structopt(long)]
    dump_functions: bool,
    /// Dump dynamic types.
    #[structopt(long)]
    dump_types: bool,
    /// Dump native functions.
    #[structopt(long)]
    dump_native_functions: bool,
    /// Dump native types.
    #[structopt(long)]
    dump_native_types: bool,
    /// Include source code references where appropriate (only available if -O debug-info=true).
    #[structopt(long)]
    with_source: bool,
    /// Enable experimental features.
    ///
    /// This makes the `std::experimental` module available to scripts.
    #[structopt(long)]
    experimental: bool,
    /// Input Rune Scripts
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// Set the given compiler option (see `--help` for available options).
    ///
    /// memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.
    ///
    /// link-checks[=<true/false>] - Perform linker checks which makes sure that called functions exist.
    ///
    /// debug-info[=<true/false>] - Enable or disable debug info.
    ///
    /// macros[=<true/false>] - Enable or disable macros (experimental).
    ///
    /// bytecode[=<true/false>] - Enable or disable bytecode caching (experimental).
    #[structopt(name = "option", short = "O", number_of_values = 1)]
    compiler_options: Vec<String>,
}

async fn try_main() -> Result<ExitCode> {
    env_logger::init();
    let args = {
        let mut args = Args::from_args();
        if args.dump {
            args.dump_unit = true;
            args.dump_stack = true;
            args.dump_functions = true;
            args.dump_types = true;
            args.dump_native_functions = true;
            args.dump_native_types = true;
        }

        if args.dump_unit {
            args.dump_unit = true;
            args.dump_instructions = true;
        }
        if args.dump_functions
            || args.dump_native_functions
            || args.dump_stack
            || args.dump_types
            || args.dump_instructions
        {
            args.dump_unit = true;
        }
        args
    };

    let mut options = rune::Options::default();
    for opt in &args.compiler_options {
        options.parse_option(opt)?;
    }



    let mut interpreter = Interpreter::new(Config {
        trace: args.trace,
        dump_unit: args.dump_unit,
        dump_instructions: args.dump_instructions,
        dump_stack: args.dump_stack,
        dump_functions: args.dump_functions,
        dump_types: args.dump_types,
        dump_native_functions: args.dump_native_functions,
        dump_native_types: args.dump_native_types,
        with_source: args.with_source,
        experimental,
        options,
    },
                                           Box::new(StandardStream::stdout(ColorChoice::Always)),
                                           Box::new(StandardStream::stderr(ColorChoice::Always)),
    )?;


    if interactive {
        InteractiveInterpreter::from(interpreter).interact().await.map(|_| ExitCode::Success)
    } else {

        interpreter.run(None).await.map(|_| ExitCode::Success)
    }
}


// Our own private ExitCode since std::process::ExitCode is nightly only.
// Note that these numbers are actually meaningful on Windows, but we don't
// care.
#[repr(i32)]
enum ExitCode {
    Success = 0,
    Failure = 1,
    VmError = 2,
}

#[tokio::main]
async fn main() {
    match try_main().await {
        Ok(exit_code) => {
            std::process::exit(exit_code as i32);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(-1);
        }
    }
}

