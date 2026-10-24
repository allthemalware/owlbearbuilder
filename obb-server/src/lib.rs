// SPDX-License-Identifier: GPL-3.0-or-later
//! The OwlbearBuilder server: an Owlbear Rodeo extension backend and the rules API it calls.

pub mod api;
pub mod assets;
pub mod http;
pub mod state;

use std::path::PathBuf;

pub use http::RunningServer;

/// Starts serving on `bind` (`127.0.0.1:0` picks a free port) with the room loaded from
/// `state_file` when it exists (saved there after every change), and dice seeded with `seed`
/// when given (tests, demos) or from the operating system. Fails when the address can't be
/// bound or the state file exists but isn't a readable room, so a typo never overwrites a
/// table's saved game.
pub fn start(
    bind: &str,
    state_file: Option<PathBuf>,
    seed: Option<u64>,
) -> Result<RunningServer, String> {
    api::start(bind, state_file, seed)
}
