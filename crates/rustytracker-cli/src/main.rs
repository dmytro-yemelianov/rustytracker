use std::process::ExitCode;

fn main() -> ExitCode {
    match rustytracker_cli::run_cli(std::env::args().skip(1)) {
        Ok(json) => {
            print!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
