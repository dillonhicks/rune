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

use anyhow::{bail, Result};
use rune::termcolor::{ColorChoice, StandardStream};
use std::env;
use std::path:: PathBuf;
use rune_interpreter::{Interpreter, Config, InteractiveInterpreter};


#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args();
    args.next();

    let mut interactive = false;

    let mut path = None;
    let mut trace = false;
    let mut dump_unit = false;
    let mut dump_instructions = false;
    let mut dump_stack = false;
    let mut dump_functions = false;
    let mut dump_types = false;
    let mut dump_native_functions = false;
    let mut dump_native_types = false;
    let mut with_source = false;
    let mut help = false;
    let mut experimental = false;

    let mut options = rune::Options::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => continue,
            "--interactive" => {
                interactive = true;
            }
            "--trace" => {
                trace = true;
            }
            "--dump" => {
                dump_unit = true;
                dump_stack = true;
                dump_functions = true;
                dump_types = true;
                dump_native_functions = true;
                dump_native_types = true;
            }
            "--dump-unit" => {
                dump_unit = true;
                dump_instructions = true;
            }
            "--dump-stack" => {
                dump_stack = true;
            }
            "--dump-instructions" => {
                dump_unit = true;
                dump_instructions = true;
            }
            "--dump-functions" => {
                dump_unit = true;
                dump_functions = true;
            }
            "--dump-types" => {
                dump_unit = true;
                dump_types = true;
            }
            "--dump-native-functions" => {
                dump_native_functions = true;
            }
            "--dump-native-types" => {
                dump_native_types = true;
            }
            "--with-source" => {
                with_source = true;
            }
            "--experimental" => {
                experimental = true;
            }
            "-O" => {
                let opt = match args.next() {
                    Some(opt) => opt,
                    None => {
                        println!("expected optimization option to `-O`");
                        return Ok(());
                    }
                };

                options.parse_option(&opt)?;
            }
            "--help" | "-h" => {
                help = true;
            }
            other if !other.starts_with('-') => {
                path = Some(PathBuf::from(other));
            }
            other => {
                println!("Unrecognized option: {}", other);
                help = true;
            }
        }
    }

    const USAGE: &str = "rune-cli [--trace] <file>";

    if help {
        println!("Usage: {}", USAGE);
        println!();
        println!("  --help, -h               - Show this help.");
        println!(
            "  --interactive            - Run the interpreter in interactive mode."
        );
        println!(
            "  --trace                  - Provide detailed tracing for each instruction executed."
        );
        println!("  --dump                   - Dump everything.");
        println!("  --dump-unit              - Dump default information about unit.");
        println!("  --dump-instructions      - Dump unit instructions.");
        println!("  --dump-stack             - Dump the state of the stack after completion. If compiled with `--trace` will dump it after each instruction.");
        println!("  --dump-functions         - Dump dynamic functions.");
        println!("  --dump-types             - Dump dynamic types.");
        println!("  --dump-native-functions  - Dump native functions.");
        println!("  --dump-native-types      - Dump native types.");
        println!("  --with-source            - Include source code references where appropriate (only available if -O debug-info=true).");
        println!("  --experimental           - Enabled experimental features.");
        println!();
        println!("Compiler options:");
        println!("  -O <option>       - Update the given compiler option.");
        println!();
        println!("Available <option> arguments:");
        println!("  memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.");
        println!("  link-checks[=<true/false>]         - Perform linker checks which makes sure that called functions exist.");
        println!("  debug-info[=<true/false>]          - Enable or disable debug info.");
        println!("  macros[=<true/false>]              - Enable or disable macros (experimental).");
        println!("  bytecode[=<true/false>]            - Enable or disable bytecode caching (experimental).");
        return Ok(());
    }

    let path = match path {
        Some(path) => path,
        None => {
            bail!("Invalid usage: {}", USAGE);
        }
    };

    let mut interpreter = Interpreter::new(Config {
        path: Some(path),
        trace,
        dump_unit,
        dump_instructions,
        dump_stack,
        dump_functions,
        dump_types,
        dump_native_functions,
        dump_native_types,
        with_source,
        experimental,
        options,
    },
                                           Box::new(StandardStream::stdout(ColorChoice::Always)),
                                           Box::new(StandardStream::stderr(ColorChoice::Always)),
    )?;


    if interactive {
        InteractiveInterpreter::from(interpreter).interact().await.map(|_| ())
    } else {
        interpreter.run(None).await.map(|_| ())
    }
}
