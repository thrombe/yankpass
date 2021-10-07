
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler, bridge::gateway::ShardManager},
    model::{
        prelude::{GuildId, GuildChannel},
        channel::Message,
    },
    framework::standard::{
        StandardFramework,
        CommandResult,
        macros::{
            command,
            group,
        },
    },
};
use tokio;
use rpassword;
use structopt::StructOpt;
use std::sync::{Arc, Mutex};
#[cfg(target_arch="x86_64")]
use enigo::{self, KeyboardControllable};

use super::crypto::CryptoHandler;

/// -d discord bot token, -r receiver: bool, -t timeout (default 64 seconds of inactivity)
#[derive(StructOpt)]
pub struct PasswordHandler {
    #[structopt(short, long)]
    discord_token: String,

    #[structopt(short, long)]
    receiver: bool,

    #[structopt(short, long, default_value = "64")]
    timeout: u64,    
}
impl PasswordHandler {
    pub fn run(self) {
        let crypto_handler = if self.receiver {Some(CryptoHandler::new())} else {None};
        start_bot(self.discord_token, self.timeout, self.receiver, crypto_handler);
    }
}

#[group]
#[commands(ping)]
struct General;

/// handler for the main program. ie interface between user, discord, crypto
struct Handler {
    receiver: bool,
    crypto_handler: Option<CryptoHandler>,
    deadline: Arc<Mutex<tokio::time::Instant>>,
    sleep_time: u64,
    quit: Arc<Mutex<bool>>,
    shard_manager: Arc<Mutex<Option<Arc<serenity::prelude::Mutex<ShardManager>>>>>,
}

impl Handler {
    async fn got_public_key(&self, strr: String, ctx: Context, message: Message) {
        let crypto_handler = CryptoHandler::new_from_public_key_pem(strr);
        loop {
            let pass = rpassword::prompt_password_stdout("enter password or q: ").unwrap();
            { // keep the block smol
                if *self.quit.lock().unwrap() {break}
            }
            if pass == "q" {
                let sm = self.shard_manager
                    .lock()
                    .unwrap().clone();
                sm.unwrap()
                    .lock()
                    .await
                    .shutdown_all()
                    .await;
                *self.quit.lock().unwrap() = true; // to shutdown the waiter loop
                    break;
            }
            self.reset_sleep();
            let encrypted_password = crypto_handler.encrypt(pass);
            message.channel_id.say(&ctx, format!("encrypted: {}", encrypted_password)).await.unwrap();
        }
    }
    
    fn got_encrypted(&self, encrypted: String) {
        #[allow(unused_variables)]
        let password = self.crypto_handler.as_ref().unwrap().decrypt(encrypted);
        #[cfg(debug_assertions)]
        dbg!(&password);
        #[cfg(target_arch="x86_64")]
        enigo::Enigo::new().key_sequence_parse(&password);
    }

    fn reset_sleep(&self) {
        *self.deadline.lock().unwrap() = tokio::time::Instant::now()+tokio::time::Duration::from_secs(self.sleep_time);
    }
}

#[async_trait]
impl EventHandler for Handler {
    // if receiver, send the private key
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        if !self.receiver {return}
        // find yankpass channel
        let mut yank_channel: Option<GuildChannel> = None;
        'loup: for guild_id in guilds {
            if let Some(channel_map) = ctx.cache.guild_channels(guild_id).await {
                for (_, channel) in channel_map {
                    if channel.name == "yankpass" {
                        yank_channel = Some(channel.clone());
                        break 'loup
                    }
                }
            }
        }
        // send public key if channel found
        if let Some(channel) = yank_channel {
            channel.say(&ctx, self.crypto_handler.as_ref().unwrap().public_key()).await.unwrap();
        } 
    }

    // check who sent the message and whats in it
    async fn message(&self, ctx: Context, new_message: Message) {
        if !new_message.author.bot {return}
        let start = "encrypted: ";
        if self.receiver && new_message.content.starts_with(start) {
            self.got_encrypted(new_message.content[start.len()..].into());
        } else if !self.receiver && new_message.content.starts_with("-----BEGIN PUBLIC KEY-----") {
            self.got_public_key(new_message.content.clone(), ctx, new_message).await;
        } else {return}
        // only reaches this if either of the first 2 conditions are correct
        self.reset_sleep();
    }
}

#[tokio::main]
async fn start_bot(token: String, sleep_time: u64, receiver: bool, crypto_handler: Option<CryptoHandler>) {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix
        .group(&GENERAL_GROUP);

    // Login with a bot token
    let deadline = Arc::new(Mutex::new(tokio::time::Instant::now()));
    let quit = Arc::new(Mutex::new(false));
    let shard_manager = Arc::new(Mutex::new(None));
    let handler = Handler {shard_manager: Arc::clone(&shard_manager), sleep_time, deadline: Arc::clone(&deadline), quit: Arc::clone(&quit), receiver, crypto_handler};
    handler.reset_sleep(); // to initially set the sleep dealine
    let mut client = Client::builder(token)
        .event_handler(handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    *shard_manager.lock().unwrap() = Some(client.shard_manager.clone());
    let shard_manager = client.shard_manager.clone(); // for shutdwon
    
    let waiter = tokio::spawn(async move {
        let sec = tokio::time::Duration::from_millis(200);
        let wait = tokio::time::sleep(sec.clone());
        let mut new_deadline = tokio::time::Instant::now();
        let mut maybe_new_deadline: tokio::time::Instant;
        tokio::pin!(wait);
        loop {
            {
                maybe_new_deadline = (*deadline.lock().unwrap()).clone();
            }
            if new_deadline < maybe_new_deadline {
                new_deadline = maybe_new_deadline;
            }
            let now = tokio::time::Instant::now();
            if new_deadline > now {
                wait.as_mut().reset(now+sec.clone());
                tokio::select!{_ = &mut wait => ()}
            } else {break}
            if *quit.lock().unwrap() {break}
        }
        shard_manager.lock().await.shutdown_all().await;
        *quit.lock().unwrap() = true; // for shutting down the password input loop
    });
    
    // start listening for events
    let bot = client.start();
    tokio::pin!(waiter, bot);
    loop {
        tokio::select! {
            Err(why) = &mut bot => println!("An error occurred while running the client: {:?}", why),
            _ = &mut waiter => break,
        }
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, &msg.content).await?;
    msg.channel_id.say(ctx, &msg.content).await?;
    Ok(())
}
