use std::{fs::canonicalize, path::PathBuf, process::Command};

fn main() {
    println!("Building the assembly sources");
    let mut command = Command::new("make");
    let mut working_directory =
        canonicalize("./asm-code").expect("Could not canonicalize ./asm-code");
    // If we are on windows, we may get an UNC path (\\?\ prefix) which rust doesn't support. We will strip this out if it is present.
    if working_directory.to_str().unwrap().starts_with("\\\\?\\") {
        working_directory = PathBuf::from(
            working_directory
                .to_str()
                .unwrap()
                .trim_start_matches("\\\\?\\"),
        );
    }
    println!(
        "Running command in {}",
        working_directory.as_os_str().to_str().unwrap()
    );
    command.current_dir(working_directory);
    command.output().expect("Failed to get output");
    let process = command.spawn().expect("Failed to compile assembly source");
    let exit_status = process.wait_with_output().expect("Failed to build").status;
    if exit_status.success() {
        println!("Built assembly sources");
    } else {
        println!("Failed to build assembly sources");
        panic!();
    }
}
