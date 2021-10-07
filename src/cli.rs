
pub type Resulte = Result<(), Box<dyn std::error::Error>>;

use structopt::StructOpt;

use super::discord::PasswordHandler;

#[derive(StructOpt)]
pub struct Opt {
    /// turns on debug prints
    // #[structopt(short = "d", long = "debug")]
    // pub debug: bool,
    
    #[structopt(subcommand)]
    pub sub: FuncOpt,
}

#[derive(StructOpt)]
pub enum FuncOpt {
    PasswordHandler(PasswordHandler),
}

impl FuncOpt {
    pub fn run(self) -> Resulte {
        match self {
            Self::PasswordHandler(func) => func.run(),
        }
        Ok(())
    }
}
