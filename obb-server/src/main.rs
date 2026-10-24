// SPDX-License-Identifier: GPL-3.0-or-later

mod cli;

fn main() {
    std::process::exit(cli::run(std::env::args().skip(1).collect()));
}
