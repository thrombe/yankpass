#![allow(dead_code)]

// mod discord;
// mod crypto;
// mod cli;

use std::{ffi::CString, fs, path::PathBuf, time::Duration};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use cxx::UniquePtr;
use rtoolbox::safe_string::SafeString;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::{UnboundedReceiver, UnboundedSender}, oneshot};

#[tokio::main]
async fn main() -> Result<()> {
    firebase_test().await?;
    Ok(())
}

pub struct UpdateDataContext(pub oneshot::Sender<Result<()>>);

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
        unsafe fn update_data(
            self: Pin<&mut Store>,
            data: *const c_char,
            done: unsafe fn(*mut c_void, ret: *const c_char),
            ctx: *mut c_void,
        );
        unsafe fn create(config_json: *const c_char) -> UniquePtr<Store>;
        unsafe fn set_listener(
            self: Pin<&mut Store>,
            callb: unsafe fn(*mut c_void, json: *const c_char, errmsg: *const c_char),
            ctx: *mut c_void,
        );
    }
}

struct Firebase {
    // i do not know if firebase stores a pointer to this
    _config_str: CString,

    app: UniquePtr<ffi::Store>,
}

impl Firebase {
    pub fn new(config_json: String) -> Result<Self> {
        let cjson = std::ffi::CString::new(config_json)?;
        let app = unsafe { ffi::create(cjson.as_ptr()) };

        let fb = Self {
            app,
            _config_str: cjson,
        };
        Ok(fb)
    }

    // TODO: fix this with rust type magic :}
    /// calling this multiple times will invalidate the other channels
    pub fn set_listenet(&mut self) -> UnboundedReceiver<Result<CString>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<CString>>();
        let ctx = Box::leak(Box::new(tx));

        unsafe {
            self.app.pin_mut().set_listener(
                |ctx_ptr: *mut ffi::c_void,
                 json: *const std::ffi::c_char,
                 errmsg: *const std::ffi::c_char| {
                    let tx = Box::from_raw(ctx_ptr as *mut UnboundedSender<Result<CString>>);
                    if let Some(json) = json.as_ref() {
                        let _ = tx.send(Ok(std::ffi::CStr::from_ptr(json).to_owned()));
                    } else if let Some(err) = errmsg.as_ref() {
                        let cstr = std::ffi::CStr::from_ptr(err).to_owned();
                        let cstr_lossy = cstr.to_string_lossy();
                        let _ = tx.send(Err(anyhow!(cstr_lossy.to_string())));
                    } else {
                        panic!();
                    }
                    let _ = Box::leak(tx);
                },
                ctx as *const _ as _,
            );

            rx
        }
    }

    pub async fn update_data(&mut self, data: &UserData) -> Result<()> {
        let data = serde_json::to_string(&data)?;
        let cdata = std::ffi::CString::new(data)?;
        let (tx, rx) = oneshot::channel::<Result<()>>();
        let ptr = Box::leak(Box::new(UpdateDataContext(tx)));
        unsafe {
            self.app.pin_mut().update_data(
                cdata.as_ptr(),
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
                ptr as *const _ as *mut ffi::c_void,
            );
        }
        rx.await??;
        Ok(())
    }

    #[cfg(target_arch="x86_64")]
    pub async fn start_receiver(mut self) -> Result<()> {
        let mut rx = self.set_listenet();

        loop {
            let timeout = tokio::time::timeout(Duration::from_secs(10), rx.recv());

            tokio::select! {
                // select panics if no patterns match. so it panics on None. which is fine
                maybe_json = timeout => {
                    match maybe_json {
                        Ok(Some(json)) => {
                            let userdata = serde_json::from_str::<UserData>(json?.to_str()?)?;
                            dbg!(&userdata);

                            use enigo::KeyboardControllable;
                            enigo::Enigo::new().key_sequence_parse(&userdata.pass);
                        },
                        Ok(None) => {
                            unreachable!();
                        },
                        Err(_) => {
                            break;
                        },
                    }
                },
            }
        }
        Ok(())
    }

    pub async fn start_sender(mut self) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SafeString>();

        tokio::task::spawn(async move {});
        let thread = tokio::task::spawn_blocking(move || -> Result<()> {
            loop {
                let pass = rpassword::prompt_password("enter passwrod: ")?;
                tx.send(SafeString::from_string(pass))?;
            }
        });
        
        loop {
            let timeout = tokio::time::timeout(Duration::from_secs(10), rx.recv());

            tokio::select! {
                // select panics if no patterns match. so it panics on None. which is fine
                maybe_p = timeout => {
                    match maybe_p {
                        Ok(Some(p)) => {
                            self.update_data(&UserData { pass: p }).await?;
                        },
                        Ok(None) => {
                            unreachable!();
                        },
                        Err(_) => {
                            break;
                        },
                    }
                },
            }
        }

        thread.abort();
        println!("\ninput timeout. -- press enter to continue --");
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
    pass: SafeString,
}



async fn firebase_test() -> Result<()> {
    let json = std::fs::read_to_string("google-services.json")?;
    let fb = Firebase::new(json)?;

    let cli = Cli::parse();

    match cli.command {
        Command::Send => {
            fb.start_sender().await?;
        },
        Command::Receive => {
            fb.start_receiver().await?;
        },
    }

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
    Send,
    Receive,
}

#[derive(Deserialize, Debug)]
struct Config {}

struct Ctx {}

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
                return Err(anyhow!("could not find a config file :/"));
            }
        };

        Ok(Self {})
    }
}
