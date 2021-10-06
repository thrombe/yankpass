
pub type Resulte = Result<(), Box<dyn std::error::Error>>;

use structopt::StructOpt;

pub enum FuncOpt {
    Func1(func),
}

impl FuncOpt {
    pub fn run(&mut self) -> Resulte {
        match self {
            Self::Func1(func) => func.run(),
        }
    }
}

pub struct func;
impl func {
    pub fn run(&self) -> Resulte {
        Ok(())
    }
}
