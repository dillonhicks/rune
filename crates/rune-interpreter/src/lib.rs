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
use rune::EmitDiagnostics as _;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use runestick::{Item, Unit, Value, VmExecution};
use std::io::{BufRead, Write};

enum Runtime {
    Initialized(Option<runestick::Vm>),
    Executing(runestick::VmExecution)
}

impl Runtime {
    pub fn execution(&mut self) -> Result<&mut runestick::VmExecution> {
        match self {
            Runtime::Initialized(vm) => {
                let execution = vm.take().unwrap().execute(&Item::of(&["main"]), ())?;
                *self = Runtime::Executing(execution);
                self.execution()
            }
            Runtime::Executing(execution) => Ok(execution),
            Runtime::Initialized(None) => {
                unreachable!()
            }
        }
    }

    pub fn vm(&self) -> Result<&runestick::Vm> {
        match self {
            Runtime::Initialized(Some(vm)) => {
                Ok(vm)
            }
            Runtime::Executing(execution) => {
                let vm= execution.vm()?;
                Ok(vm)
            }
            Runtime::Initialized(None) => {
                unreachable!()
            }
        }
    }

    pub fn vm_mut(&mut self) -> Result<&mut runestick::Vm >{
        match self {
            Runtime::Initialized(Some(vm)) => {
                Ok(vm)
            }
            Runtime::Executing(execution) => {
                let vm= execution.vm_mut()?;
                Ok(vm)
            }
            Runtime::Initialized(None) => {
                unreachable!()
            }
        }
    }
}

pub struct Interpreter {
    config: Config,
    sources: rune::Sources,
    context: Arc<runestick::Context>,
    unit: Arc<Unit>,
    stdout: Box<dyn rune::termcolor::WriteColor>,
    stderr: Box<dyn rune::termcolor::WriteColor>,
}

impl Interpreter {
    pub fn new(config: Config, stdout: Box<dyn rune::termcolor::WriteColor>,mut  stderr: Box<dyn rune::termcolor::WriteColor>) -> Result<Interpreter> {
        

    let bytecode_path = config.path.as_ref().map(|p| p.with_extension("rnc"));
    let mut context = rune::default_context()?;

    if config.experimental {
        context.install(&rune_macros::module()?)?;
    }

    let context = Arc::new(context);
    let mut sources = rune::Sources::new();
    let mut warnings = rune::Warnings::new();

    let use_cache = config.options.bytecode && should_cache_be_used(&config.path, &bytecode_path)?;
    let maybe_unit = if use_cache {
        let bytecode_path = bytecode_path.clone().unwrap();
        let f = fs::File::open(&bytecode_path)?;
        match bincode::deserialize_from::<_, Unit>(f) {
            Ok(unit) => {
                log::trace!("using cache: {}", bytecode_path.display());
                Some(Arc::new(unit))
            }
            Err(e) => {
                log::error!("failed to deserialize: {}: {}", bytecode_path.display(), e);
                None
            }
        }
    } else {
        None
    };

    let unit = match maybe_unit {
        Some(unit) => unit,
        None => {
            let path = config.path.clone().unwrap();
            log::trace!("building file: {}", path.display());

            let unit =
                match rune::load_path(&*context, &config.options, &mut sources, &path, &mut warnings) {
                    Ok(unit) => unit,
                    Err(error) => {
                        let mut writer = StandardStream::stderr(ColorChoice::Always);
                        error.emit_diagnostics(&mut stderr, &sources)?;
                         bail!("aborting due to load errors");
                    }
                };

            if config.options.bytecode {
                let bytecode_path = bytecode_path.clone().unwrap();
                log::trace!("serializing cache: {}", bytecode_path.display());
                let f = fs::File::create(&bytecode_path)?;
                bincode::serialize_into(f, &unit)?;
            }

            Arc::new(unit)
        }
    };

    if !warnings.is_empty() {
        warnings.emit_diagnostics( &mut stderr, &sources)?;
    }

       Ok( Interpreter {
           config,
            sources,
            context,
            unit,
            stdout,
            stderr,
        })
        
    }

    pub async fn run(&mut self, target: Option<Item>) -> Result<Option<Value>> {

        macro_rules! println {
            ($($arg:tt)*) => {{
                writeln!(&mut *self.stdout, $($arg)*)?;
            }}
        };
        macro_rules! print {
            ($($arg:tt)*) => {{
                write!(&mut *self.stdout, $($arg)*)?;
            }}
        };

        let mut vm = runestick::Vm::new(self.context.clone(), self.unit.clone());

        if self.config.dump_native_functions {
            println!("# functions");
    
            for (i, (hash, f)) in self.context.iter_functions().enumerate() {
                println!("{:04} = {} ({})", i, f, hash);
            }
        }
    
        if self.config.dump_native_types {
            println!("# types");
    
            for (i, (hash, ty)) in self.context.iter_types().enumerate() {
                println!("{:04} = {} ({})", i, ty, hash);
            }
        }
    
        if self.config.dump_unit {
    
            let unit = &self.unit;
    
            if self.config.dump_instructions {
                println!("# instructions");
    
                let mut first_function = true;
    
                for (n, inst) in unit.iter_instructions().enumerate() {
    
                    let debug = unit.debug_info().and_then(|d| d.instruction_at(n));
    
                    if let Some((hash, signature)) = unit.debug_info().and_then(|d| d.function_at(n)) {
                        if first_function {
                            first_function = false;
                        } else {
                            println!();
                        }
    
                        println!("fn {} ({}):", signature, hash);
                    }
    
                    if self.config.with_source {
                        let sources = &self.sources;
                        if let Some((source, span)) =
                            debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                        {
                            if let Some((count, line)) =
                                rune::diagnostics::line_for(source.as_str(), span)
                            {
                                println!(
                                    "  {}:{: <3} - {}",
                                    source.name(),
                                    count + 1,
                                    line.trim_end()
                                );
                            }
                        }
                    }
    
                    if let Some(label) = debug.and_then(|d| d.label.as_ref()) {
                        println!("{}:", label);
                    }
    
                    print!("  {:04} = {}", n, inst);
    
                    if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
                        print!(" // {}", comment);
                    }
    
                    println!();
                }
            }
    
            let mut functions = unit.iter_functions().peekable();
            let mut types = unit.iter_types().peekable();
            let mut strings = unit.iter_static_strings().peekable();
            let mut keys = unit.iter_static_object_keys().peekable();
    
            if self.config.dump_functions && functions.peek().is_some() {
                println!("# dynamic functions");
    
                for (hash, kind) in functions {
                    if let Some(signature) = unit.debug_info().and_then(|d| d.functions.get(&hash)) {
                        println!("{} = {}", hash, signature);
                    } else {
                        println!("{} = {}", hash, kind);
                    }
                }
            }
    
            if self.config.dump_types && types.peek().is_some() {
                println!("# dynamic types");
    
                for (hash, ty) in types {
                    println!("{} = {}", hash, ty.value_type);
                }
            }
    
            if strings.peek().is_some() {
                println!("# strings");
    
                for string in strings {
                    println!("{} = {:?}", string.hash(), string);
                }
            }
    
            if keys.peek().is_some() {
                println!("# object keys");
    
                for (hash, keys) in keys {
                    println!("{} = {:?}", hash, keys);
                }
            }
        }
    
        let last = std::time::Instant::now();

       let mut execution = vm.execute(&target.unwrap_or_else(|| Item::of(&["main"])), ())?;
    
        let result = if self.config.trace {
            match do_trace(&mut execution, &self.sources, self.config.dump_stack, self.config.with_source).await {
                Ok(value) => Ok(value),
                Err(TraceError::Io(io)) => return Err(io.into()),
                Err(TraceError::VmError(vm)) => Err(vm),
            }
        } else {
            execution.async_complete().await
        };
    
        let errored;
    
        let value: Option<Value> = match result {
            Ok(result) => {
                let duration = std::time::Instant::now().duration_since(last);
                println!("== {:?} ({:?})", result, duration);
                errored = None;
                Some(result)
            }
            Err(error) => {
                let duration = std::time::Instant::now().duration_since(last);
                println!("== ! ({}) ({:?})", error, duration);
                errored = Some(error);
                None
            }
        };
    
        if self.config.dump_stack {
            println!("# full stack dump after halting");
    
            let vm = execution.vm_mut()?;
            let frames = vm.call_frames();
            let stack = vm.stack();
    
            let mut it = frames.iter().enumerate().peekable();
    
            while let Some((count, frame)) = it.next() {
                let stack_top = match it.peek() {
                    Some((_, next)) => next.stack_bottom(),
                    None => stack.stack_bottom(),
                };
    
                let values = stack
                    .get(frame.stack_bottom()..stack_top)
                    .expect("bad stack slice");
    
                println!("  frame #{} (+{})", count, frame.stack_bottom());
    
                if values.is_empty() {
                    println!("    *empty*");
                }
    
                for (n, value) in stack.iter().enumerate() {
                    println!("{}+{} = {:?}", frame.stack_bottom(), n, value);
                }
            }
    
            // NB: print final frame
            println!("  frame #{} (+{})", frames.len(), stack.stack_bottom());
    
            let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");
    
            if values.is_empty() {
                println!("    *empty*");
            }
    
            for (n, value) in values.iter().enumerate() {
                println!("    {}+{} = {:?}", stack.stack_bottom(), n, value);
            }
        }
    
        if let Some(error) = errored {
            error.emit_diagnostics( &mut self.stderr, &self.sources)?;
        }

        Ok(value)
    }



}

pub struct InteractiveInterpreter {
    interpreter: Interpreter,
}

impl InteractiveInterpreter {
    pub const fn new(interpreter: Interpreter) -> Self {
        InteractiveInterpreter {
            interpreter
        }
    }

    pub async fn interact(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut cnt = 0usize;

            loop {
                let mut buffer = String::new();
                let mut stdout = std::io::stdout();
                stdout.lock().write_all(format!("In[{}]: ", cnt).as_bytes()).expect("could not write to stdout");
                stdout.lock().flush();
                std::io::stdin().lock().read_line(&mut buffer).expect("could not read from stdin");
                stdout.write_all(b"\n").expect("could not write to stdout");
                tx.send(buffer).expect("could not write to input channel");
                cnt += 1;
            }
        });

        let mut cnt = 0usize;
        loop {
            match rx.recv() {
                Ok(input) => {
                    let output = self.eval(cnt,input).await?;
                    write!(self.interpreter.stdout, "Out[{}]: ",  cnt);
                    if let Some(output) = output {
                        write!(self.interpreter.stdout, "{:?}", cnt)?;
                        cnt+=1;
                    }
                    writeln!(self.interpreter.stdout, "");
                },
                Err(err) => writeln!(self.interpreter.stderr, "could not read from input channel")?,
            }
        }

        Ok(())
    }


    pub async fn eval(&mut self, uid: usize, source: String) -> Result<Option<Value>> {

        let fn_name = format!("eval_expression_{}", uid);

        let source = format!(r#"
                    async fn {}() {{
                {}
            }}
            "#,  fn_name, source);

        let mut warnings = rune::Warnings::new();

        let mut sources = rune::Sources::new();
        sources.insert_default(runestick::Source::new(format!("eval{}", uid), source ));

        let unit = match rune::load_sources(&*self.interpreter.context, &self.interpreter.config.options, &mut sources, &mut warnings) {
            Ok(unit) => unit,
            Err(error) => {
                error.emit_diagnostics(&mut self.interpreter.stderr, &sources)?;
                return Ok(None)
            }
        };

        self.interpreter.unit = Arc::new(unit);

        self.interpreter.run(Some(Item::of(&[fn_name]))).await
    }
}


impl std::convert::From<Interpreter> for InteractiveInterpreter {
    fn from(interpreter: Interpreter) -> Self {
        InteractiveInterpreter::new(interpreter)
    }
}

pub struct Config {
    pub path: Option<PathBuf>,
    pub trace : bool,
    pub dump_unit : bool,
    pub dump_instructions : bool,
    pub dump_stack : bool,
    pub dump_functions : bool,
    pub dump_types : bool,
    pub dump_native_functions : bool,
    pub dump_native_types : bool,
    pub with_source : bool,
    pub experimental : bool,
    pub options: rune::Options,
}


enum TraceError {
    Io(std::io::Error),
    VmError(runestick::VmError),
}

impl From<std::io::Error> for TraceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Perform a detailed trace of the program.
async fn do_trace(
    execution: &mut VmExecution,
    sources: &rune::Sources,
    dump_stack: bool,
    with_source: bool,
) -> Result<Value, TraceError> {
    use std::io::Write as _;
    let out = std::io::stdout();

    let mut current_frame_len = execution
        .vm()
        .map_err(TraceError::VmError)?
        .call_frames()
        .len();

    loop {
        {
            let vm = execution.vm().map_err(TraceError::VmError)?;
            let mut out = out.lock();

            if let Some((hash, signature)) =
                vm.unit().debug_info().and_then(|d| d.function_at(vm.ip()))
            {
                writeln!(out, "fn {} ({}):", signature, hash)?;
            }

            let debug = vm
                .unit()
                .debug_info()
                .and_then(|d| d.instruction_at(vm.ip()));

            if with_source {
                if let Some((source, span)) =
                    debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                {
                    if let Some((count, line)) = rune::diagnostics::line_for(source.as_str(), span)
                    {
                        writeln!(
                            out,
                            "  {}:{: <3} - {}",
                            source.name(),
                            count + 1,
                            line.trim_end()
                        )?;
                    }
                }
            }

            if let Some(inst) = debug {
                if let Some(label) = &inst.label {
                    writeln!(out, "{}:", label)?;
                }
            }

            if let Some(inst) = vm.unit().instruction_at(vm.ip()) {
                write!(out, "  {:04} = {}", vm.ip(), inst)?;
            } else {
                write!(out, "  {:04} = *out of bounds*", vm.ip())?;
            }

            if let Some(inst) = debug {
                if let Some(comment) = &inst.comment {
                    write!(out, " // {}", comment)?;
                }
            }

            writeln!(out,)?;
        }

        let result = match execution.async_step().await {
            Ok(result) => result,
            Err(e) => return Err(TraceError::VmError(e)),
        };

        let mut out = out.lock();

        if dump_stack {
            let vm = execution.vm().map_err(TraceError::VmError)?;
            let frames = vm.call_frames();

            let stack = vm.stack();

            if current_frame_len != frames.len() {
                if current_frame_len < frames.len() {
                    println!("=> frame {} ({}):", frames.len(), stack.stack_bottom());
                } else {
                    println!("<= frame {} ({}):", frames.len(), stack.stack_bottom());
                }

                current_frame_len = frames.len();
            }

            let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");

            if values.is_empty() {
                println!("    *empty*");
            }

            for (n, value) in values.iter().enumerate() {
                writeln!(out, "    {}+{} = {:?}", stack.stack_bottom(), n, value)?;
            }
        }

        if let Some(result) = result {
            break Ok(result);
        }
    }

    
}

/// Test if path `a` is newer than path `b`.
fn should_cache_be_used(source: &Option<PathBuf>, cached: &Option<PathBuf>) -> io::Result<bool> {
    if let (Some(source), Some(cached)) = (source, cached) {
    let source = fs::metadata(source)?;

    let cached = match fs::metadata(cached) {
        Ok(cached) => cached,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };

    Ok(source.modified()? < cached.modified()?)
    } else {
        Ok(false)
    }
}
