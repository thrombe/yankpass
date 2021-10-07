#![allow(dead_code)]

mod discord;
mod crypto;
mod cli;

use structopt::StructOpt;

use cli::Opt;

fn main() {
    let opt = Opt::from_args();
    opt.sub.run().unwrap();
}
