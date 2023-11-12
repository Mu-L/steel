extern crate steel;
extern crate steel_derive;
extern crate steel_repl;

use steel::steel_vm::engine::Engine;
use steel_doc::walk_dir;
use steel_repl::repl::repl_base;

use std::path::PathBuf;
use std::process;
use std::{error::Error, fs};

use clap::Parser;

/// Steel Interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// What action to perform on this file, the absence of a subcommand indicates that the given file (if any)
    /// will be run as the entrypoint
    #[clap(subcommand)]
    action: Option<EmitAction>,

    /// The existence of this argument indicates whether we want to run the repl, or interpret this file
    default_file: Option<PathBuf>,

    /// Arguments to the input file
    arguments: Vec<String>,
}

#[derive(clap::Subcommand, Debug)]
enum EmitAction {
    /// Output a debug display of the fully transformed bytecode
    Bytecode { default_file: Option<PathBuf> },
    /// Print a debug display of the fully expanded AST
    Ast { default_file: Option<PathBuf> },
    /// Enter the repl with the given file loaded
    Interactive {
        default_file: Option<PathBuf>,
        arguments: Vec<String>,
    },
    /// Tests the module - only tests modules which provide values
    Test { default_file: Option<String> },
    /// Generate the documentation for a file
    Doc { default_file: Option<PathBuf> },
}

pub fn run(clap_args: Args) -> Result<(), Box<dyn Error>> {
    let mut vm = Engine::new();

    // let mut vm = Engine::top_level_load_from_bootstrap(include_bytes!(concat!(
    //     env!("OUT_DIR"),
    //     "/bootstrap.bin"
    // )));

    vm.register_value("std::env::args", steel::SteelVal::ListV(vec![].into()));

    match clap_args {
        Args {
            default_file: None,
            action: None,
            ..
        } => {
            repl_base(vm)?;
            Ok(())
        }

        Args {
            default_file: Some(path),
            action: None,
            arguments,
        } => {
            vm.register_value(
                "std::env::args",
                steel::SteelVal::ListV(
                    arguments
                        .into_iter()
                        .map(|x| steel::SteelVal::StringV(x.into()))
                        .collect(),
                ),
            );

            let contents = fs::read_to_string(&path)?;
            let res = vm.compile_and_run_raw_program_with_path(&contents, path.clone());

            if let Err(e) = res {
                e.emit_result(path.to_str().unwrap(), &contents);
                // process::exit(1);

                return Err(Box::new(e));
            }

            Ok(())
        }

        Args {
            default_file: None,
            action: Some(EmitAction::Test { default_file }),
            ..
        } => {
            let file_or_current_dir: String = default_file.unwrap_or(".".to_string());
            if let Some(path) = PathBuf::from(file_or_current_dir).to_str() {
                let mut vm = Engine::new();
                vm.register_value(
                    "std::env::args",
                    steel::SteelVal::ListV(vec![path.to_string().into()].into()),
                );
                let test_script = include_str!("../cogs/test-runner.scm");
                if let Err(e) = vm.run(test_script) {
                    e.emit_result(path, &test_script);
                    return Err(Box::new(e));
                }
            }
            Ok(())
        }
        Args {
            default_file: None,
            action: Some(EmitAction::Doc {
                default_file: Some(path),
            }),
            ..
        } => {
            // todo!()

            let mut writer = std::io::BufWriter::new(std::io::stdout());

            walk_dir(&mut writer, path, &mut vm)?;

            Ok(())

            // todo!()
        }

        Args {
            default_file: None,
            action:
                Some(EmitAction::Bytecode {
                    default_file: Some(path),
                }),
            ..
        } => {
            let contents = fs::read_to_string(&path)?;

            let program = vm.emit_raw_program(&contents, path.clone());

            match program {
                Ok(program) => {
                    vm.debug_print_build(path.to_str().unwrap().to_string(), program)
                        .unwrap();
                }
                Err(e) => e.emit_result(path.to_str().unwrap(), &contents),
            }

            Ok(())
        }

        Args {
            default_file: None,
            action: Some(EmitAction::Ast {
                default_file: Some(path),
            }),
            ..
        } => {
            let contents = fs::read_to_string(path.clone())?;

            let res = vm.emit_fully_expanded_ast_to_string(&contents, Some(path.clone()));

            match res {
                Ok(ast) => println!("{ast}"),
                Err(e) => e.emit_result(path.to_str().unwrap(), &contents),
            }

            Ok(())
        }

        Args {
            default_file: None,
            action:
                Some(EmitAction::Interactive {
                    default_file: Some(path),
                    arguments: _,
                }),
            ..
        } => {
            let core_libraries = &[steel::stdlib::PRELUDE];

            for core in core_libraries {
                let res = vm.compile_and_run_raw_program(core);
                if let Err(e) = res {
                    eprintln!("{e}");
                    return Ok(());
                }
            }

            let contents =
                fs::read_to_string(&path).expect("Something went wrong reading the file");
            let res = vm.compile_and_run_raw_program_with_path(&contents, path.clone());

            if let Err(e) = res {
                e.emit_result(path.to_str().unwrap(), &contents);
            }

            repl_base(vm)?;
            Ok(())
        }

        _ => {
            repl_base(vm)?;
            Ok(())
        }
    }
}

pub fn finish(result: Result<(), std::io::Error>) -> ! {
    let code = match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!(
                "{}: {}",
                std::env::args().next().unwrap_or_else(|| "steel".into()),
                e
            );
            1
        }
    };

    process::exit(code);
}

#[test]
fn test_runner() {
    let args = Args {
        action: None,
        default_file: Some(PathBuf::from("cogs/test-runner.scm")),
        arguments: vec!["cogs/".to_string()],
    };

    run(args).unwrap()
}

#[test]
fn r5rs_test_suite() {
    let args = Args {
        action: None,
        default_file: Some(PathBuf::from("cogs/r5rs.scm")),
        arguments: vec![],
    };

    run(args).unwrap()
}

#[test]
fn r7rs_test_suite() {
    let args = Args {
        action: None,
        default_file: Some(PathBuf::from("cogs/r7rs.scm")),
        arguments: vec![],
    };

    run(args).unwrap()
}

#[test]
fn r7rs_benchmark_test_suite() {
    let benches = &[
        "r7rs-benchmarks/scheme.scm",
        "r7rs-benchmarks/simplex.scm",
        "r7rs-benchmarks/array1.scm",
        "r7rs-benchmarks/triangl.scm",
    ];

    for bench in benches {
        let args = Args {
            action: None,
            default_file: Some(PathBuf::from(bench)),
            arguments: vec![],
        };

        run(args).unwrap();
    }
}
