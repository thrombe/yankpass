#![allow(dead_code)]

// mod discord;
// mod crypto;
// mod cli;

use std::{
    fs,
    path::PathBuf, ffi::CString,
};

use anyhow::{Result, anyhow, Context};
use cxx::UniquePtr;
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};

#[tokio::main]
async fn main() -> Result<()> {
    firebase_test().await?;
    Ok(())
}

pub struct UpdateDataContext(pub tokio::sync::oneshot::Sender<Result<()>>);

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type UpdateDataContext;
    }
    unsafe extern "C++" {
        include!("yankpass/include/test.h");

        type c_void;

        type Store;
        fn drop(self: Pin<&mut Store>);
        unsafe fn update_data(self: Pin<&mut Store>, data: *const c_char,
            done: unsafe fn(*mut c_void, ret: *const c_char),
            ctx: *mut c_void,
        );
        unsafe fn create(config_json: *const c_char) -> UniquePtr<Store>;
    }
}

struct Firebase {
    // i do not know if firebase stores a pointer to this
    _config_str: CString,

    app: UniquePtr<ffi::Store>,
}

impl Firebase {
    fn new(config_json: String) -> Result<Self> {
        let cjson = std::ffi::CString::new(config_json)?;
        let app = unsafe {ffi::create(cjson.as_ptr())};

        let fb = Self {
            app,
            _config_str: cjson,
        };
        Ok(fb)
    }

    async fn update_data(&mut self, data: &UserData) -> Result<()> {
        let data = serde_json::to_string(&data)?;
        let cdata = std::ffi::CString::new(data)?;
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<()>>();
        let ptr = Box::leak(Box::new(UpdateDataContext(tx)));
        unsafe {
            self.app.pin_mut()
                .update_data(cdata.as_ptr(),
                |ctx, val| {
                        let b = Box::from_raw(ctx as *mut UpdateDataContext);
                        let val = val.as_ref();
                        if let Some(v) = val {
                            let st = std::ffi::CStr::from_ptr(v);
                            let _ = b.0.send(Err(anyhow!(st.to_str().unwrap())));
                        } else {
                            let _ = b.0.send(Ok(()));
                        }
                },
                    ptr as *const _ as *mut ffi::c_void
            );
        }
        rx.await??;
        Ok(())
    }
}

impl Drop for Firebase {
    fn drop(&mut self) {
        self.app.pin_mut().drop();
    }
}


#[derive(Serialize, Deserialize, Debug)]
struct UserData {
    string: String,
}

async fn firebase_test() -> Result<()> {
    let json = std::fs::read_to_string("google-services.json")?;
    let mut fb = Firebase::new(json)?;

    fb.update_data(&UserData { string: "lamo take this".into() }).await?;

    std::thread::sleep(std::time::Duration::from_millis(1000));

    Ok(())
}






#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Specify a custom config directory
    #[arg(short, long)]
    config_dir: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Add paths to the current profile
    Add {
        #[clap(required = true)]
        src: Vec<String>,
    },
}

#[derive(Deserialize, Debug)]
struct Config {
}

struct Ctx {
}

impl Ctx {
    fn new(cli: &Cli) -> Result<Self> {
        let config_dir = {
            let config_dir = dirs::config_dir()
                .context("Could not find config dir.")?
                .join("yankpass");

            if cli.config_dir.is_none() && !config_dir.exists() {
                fs::create_dir(&config_dir)?;
            }

            cli.config_dir
                .as_ref()
                .map(shellexpand::tilde)
                .map(|s| s.to_string())
                .map(PathBuf::from)
                .map(|p| p.canonicalize())
                .unwrap_or(Ok(config_dir))?
        };

        let _conf: Config = {
            let config_file_path = config_dir.join("config.toml");
            if config_file_path.exists() {
                let contents = std::fs::read_to_string(config_file_path)?;
                toml::from_str(&contents)?
            } else {
                return Err(anyhow!(
                    "could not find a config file :/"
                ));
            }
        };

        Ok(Self {})
    }
}

