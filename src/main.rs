use std::{ffi::CString, fs, io::Write, path::PathBuf, time::Duration};

use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, KeyInit,
};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use cxx::UniquePtr;
use rtoolbox::safe_string::SafeString;
use serde::{Deserialize, Serialize};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};

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
    pub fn set_listener(&mut self) -> UnboundedReceiver<Result<CString>> {
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

    #[cfg(target_arch = "x86_64")]
    pub async fn start_receiver(mut self, cipher: Aes256Gcm) -> Result<()> {
        let mut rx = self.set_listener();

        loop {
            let timeout = tokio::time::timeout(Duration::from_secs(10), rx.recv());

            tokio::select! {
                // select panics if no patterns match. so it panics on None. which is fine
                maybe_json = timeout => {
                    match maybe_json {
                        Ok(Some(json)) => {
                            let userdata = serde_json::from_str::<UserData>(json?.to_str()?)?;
                            dbg!(&userdata);

                            let nonce = aes_gcm::Nonce::<<aes_gcm::Aes256Gcm as aes_gcm::AeadCore>::NonceSize>::from_slice(&userdata.nonce);
                            let plaintext = cipher.decrypt(nonce, &userdata.ciphertext[..]).ok().context("failed to decrypt")?;

                            use enigo::KeyboardControllable;
                            enigo::Enigo::new().key_sequence_parse(String::from_utf8(plaintext)?.as_ref());
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

    pub async fn start_sender(mut self, cipher: Aes256Gcm) -> Result<()> {
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
                            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                            let ciphertext = cipher.encrypt(&nonce, p.as_bytes()).ok().context("failed to encrypt")?;
                            self.update_data(&UserData {
                                ciphertext: ciphertext.into_boxed_slice(),
                                nonce: nonce.to_vec().into_boxed_slice(),
                            }).await?;
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
    // TODO: these are serialized very inefficiently
    ciphertext: Box<[u8]>,
    nonce: Box<[u8]>,
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
    NewKey { output_file: String },
}

#[derive(Deserialize, Debug)]
struct Config {
    username: String,
    firebase_json_path: String,
    key_path: String,
}

impl Config {
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

        let conf: Config = {
            let config_file_path = config_dir.join("config.toml");
            if config_file_path.exists() {
                let contents = std::fs::read_to_string(config_file_path)?;
                toml::from_str(&contents)?
            } else {
                return Err(anyhow!("could not find a config file :/"));
            }
        };

        Ok(conf)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Command::NewKey { output_file } = &cli.command {
        let output_file = {
            let tilde = shellexpand::tilde(&output_file).to_string();
            PathBuf::from(tilde)
        };
        if output_file.exists() {
            return Err(anyhow!("a file already exists at {:?}", output_file));
        }

        let key = Aes256Gcm::generate_key(OsRng);
        let slice = key.as_slice();

        fs::File::create(output_file)?.write_all(slice)?;
        return Ok(());
    }

    let conf = Config::new(&cli)?;

    let json = std::fs::read_to_string(shellexpand::tilde(&conf.firebase_json_path).as_ref())?;
    let fb = Firebase::new(json)?;

    let key_vec = fs::read(shellexpand::tilde(&conf.key_path).as_ref()).unwrap();
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key_vec.as_slice());
    let cipher = Aes256Gcm::new(key);

    match &cli.command {
        Command::Send => {
            fb.start_sender(cipher).await?;
        }
        Command::Receive => {
            fb.start_receiver(cipher).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
