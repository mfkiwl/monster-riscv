use std::env;
use std::process::Command;

fn boolector() {
    Command::new("git")
        .current_dir(env::var("OUT_DIR").unwrap())
        .arg("clone")
        .arg("https://github.com/Boolector/boolector.git")
        .output();

    Command::new("contrib/setup-lingeling.sh")
        .current_dir(format!("{}/boolector", env::var("OUT_DIR").unwrap()))
        .output();

    Command::new("contrib/setup-btor2tools.sh")
        .current_dir(format!("{}/boolector", env::var("OUT_DIR").unwrap()))
        .output();

    Command::new("./configure.sh")
        .current_dir(format!("{}/boolector", env::var("OUT_DIR").unwrap()))
        .arg("--shared")
        .output();

    Command::new("make")
        .current_dir(format!("{}/boolector/build", env::var("OUT_DIR").unwrap()))
        .output();

    // for mac
    // DYLD_LIBRARY_PATH

    // for linux
    // LD_LIBRARY_PATH

    // the "-L" flag
    println!(
        "cargo:rustc-link-search={}/boolector/build/lib",
        env::var("OUT_DIR").unwrap()
    );
}

fn main() {
    boolector();
}
