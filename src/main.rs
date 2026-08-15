use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(autumnk_gwt::cli::run(std::env::args().skip(1).collect()))
}
