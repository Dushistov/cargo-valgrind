//! The `cargo-valgrind` executable.
#![forbid(unsafe_code)]
#![deny(clippy::correctness)]
#![warn(
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::nursery,
    clippy::pedantic,
    clippy::clone_on_ref_ptr,
    clippy::decimal_literal_representation,
    clippy::float_cmp_const,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::unwrap_used,
    clippy::cargo_common_metadata,
    clippy::used_underscore_binding
)]

mod driver;
mod output;
mod panic;
mod valgrind;

use colored::Colorize as _;
use std::env;
use std::process;

/// Part of the output message of `valgrind` if a possible stack overflow is
/// detected.
const STACK_OVERFLOW: &str = "main thread stack using the --main-stacksize= flag";

fn main() {
    panic::replace_hook();

    let number_of_arguments = || env::args_os().skip(1).count();
    let help_requested = || env::args_os().any(|arg| arg == "--help" || arg == "-h");
    let is_cargo_subcommand = || env::args_os().nth(1).map_or(false, |arg| arg == "valgrind");
    if number_of_arguments() == 0 || help_requested() {
        let text = format!(
            "cargo valgrind {version}\n\
            {authors}\n\
            Analyze your Rust binary for memory errors\n\
            \n\
            This program is a cargo subcommand, i.e. it integrates with the \
            normal cargo workflow. You specify this subcommand and another \
            \"target\", what valgrind should do. For example: `cargo valgrind \
            run` will do the same thing as `cargo run` (i.e. compile and run \
            your binary), but the execution will be done using valgrind. \
            Similarly to execute the tests, simply use `cargo valgrind test`.",
            version = env!("CARGO_PKG_VERSION"),
            authors = env!("CARGO_PKG_AUTHORS").replace(':', ", "),
        );
        #[cfg(feature = "textwrap")]
        let text = textwrap::wrap(
            textwrap::dedent(&text).trim_start(),
            textwrap::Options::with_termwidth(),
        )
        .join("\n");
        println!("{text}");
    } else if is_cargo_subcommand() {
        if !driver::driver().expect("Could not execute subcommand") {
            process::exit(200);
        }
    } else {
        // we are running as the cargo runner, therefore everything except the
        // first argument is the command to execute.
        let command = env::args_os().skip(1);

        let exit_code = match valgrind::execute(command) {
            Ok(valgrind::xml::Output {
                errors: Some(errors),
                ..
            }) => {
                output::display_errors(&errors);
                127
            }
            Ok(_) => 0,
            Err(e @ valgrind::Error::MalformedOutput(..)) => {
                panic_with!(e);
            }
            Err(valgrind::Error::ValgrindFailure(output)) if output.contains(STACK_OVERFLOW) => {
                output::display_stack_overflow(&output);
                134 // default exit code for stack overflows
            }
            Err(e) => {
                eprintln!("{}: {}", "error".red().bold(), e);
                1
            }
        };
        process::exit(exit_code);
    }
}
