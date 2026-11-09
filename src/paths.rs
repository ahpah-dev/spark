//! Where Spark keeps your games.
//!
//! If there is a `games` folder next to the program (like in the zip you get from `package.ps1`), we use
//! the folders next to the program, so your games are always in the same place however you start it.
//! Otherwise (when developing) we use the current folder.

use std::path::PathBuf;

/// The folder that holds `games` and `exports`.
pub fn data_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("games").is_dir() {
                return dir.to_path_buf();
            }
        }
    }
    PathBuf::from(".")
}

pub fn games_dir() -> PathBuf {
    data_root().join("games")
}

pub fn exports_dir() -> PathBuf {
    data_root().join("exports")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn while_developing_we_use_the_current_folder() {
        // The test program lives in target/..., which has no "games" folder next to it.
        assert_eq!(data_root(), PathBuf::from("."));
        assert!(games_dir().ends_with("games"));
        assert!(exports_dir().ends_with("exports"));
    }
}
