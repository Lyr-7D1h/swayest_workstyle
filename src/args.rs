use std::{env, process};

/// Exit program if help arg given
pub fn help() {
    for arg in env::args() {
        if arg == "-h" || arg == "--help" {
            println!("Swayest Workstyle.\nThis tool will rename workspaces to the icons configured.\nConfig can be found in $HOME/.config/sworkstyle");
            process::exit(0);
        }
    }
}
