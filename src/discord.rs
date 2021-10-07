
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
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
use std::env;
use rpassword;
use std::sync::{Arc, Mutex};
#[cfg(target_arch="x86_64")]
use enigo::{self, KeyboardControllable};

use super::crypto::CryptoHandler;

pub fn test1() {
    if &env::args().collect::<Vec<String>>()[2] == "0" {start_bot(64, true, Some(CryptoHandler::new()))}; // pc
    if &env::args().collect::<Vec<String>>()[2] == "1" {start_bot(64, false, None)}; // phone
}

#[group]
#[commands(ping)]
struct General;

/// handler for the main program. ie interface between user, discord, crypto
struct Handler {
    token: String,
    receiver: bool,
    crypto_handler: Option<CryptoHandler>,
    deadline: Arc<Mutex<tokio::time::Instant>>,
    sleep_time: u64,
    quit: Arc<Mutex<bool>>,
}


impl Handler {
    pub async fn got_public_key(&self, strr: String, ctx: Context, message: Message) {
        let crypto_handler = CryptoHandler::new_from_public_key_pem(strr);
        loop {
            let pass = rpassword::prompt_password_stdout("enter password or something: ").unwrap();
            { // keep the block smol
                if *self.quit.lock().unwrap() {break}
            }
            if pass == "q" {break}
            self.reset_sleep();
            let encrypted_password = crypto_handler.encrypt(pass);
            message.channel_id.say(&ctx, format!("encrypted: {}", encrypted_password)).await.unwrap();
        }
    }
    
    pub fn got_encrypted(&self, encrypted: String) {
        let password = self.crypto_handler.as_ref().unwrap().decrypt(encrypted);
        #[cfg(debug_assertions)]
        dbg!(&password);
        #[cfg(target_arch="x86_64")]
        enigo::Enigo::new().key_sequence_parse(&password);
    }

    pub fn reset_sleep(&self) {
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
async fn start_bot(sleep_time: u64, receiver: bool, crypto_handler: Option<CryptoHandler>) {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix
        .group(&GENERAL_GROUP);

    // Login with a bot token
    let token = &env::args().collect::<Vec<String>>()[1];
    let deadline = Arc::new(Mutex::new(tokio::time::Instant::now()));
    let quit = Arc::new(Mutex::new(false));
    let handler = Handler {token: token.to_owned(), sleep_time, deadline: Arc::clone(&deadline), quit: Arc::clone(&quit), receiver, crypto_handler};
    handler.reset_sleep(); // to initially set the sleep dealine
    let mut client = Client::builder(token)
        .event_handler(handler)
        .framework(framework)
        .await
        .expect("Error creating client");
    let shard_manager = client.shard_manager.clone(); // for shutdwon
    
    let waiter = tokio::spawn(async move {
        let wait = tokio::time::sleep(tokio::time::Duration::from_secs(1)); // this will be reset in the first loop
        let mut new_deadline = tokio::time::Instant::now();
        let mut maybe_new_deadline: tokio::time::Instant;
        tokio::pin!(wait);
        loop {
            tokio::select!{_ = &mut wait => ()}
            {
                maybe_new_deadline = (*deadline.lock().unwrap()).clone();
            }
            if new_deadline != maybe_new_deadline {
                new_deadline = maybe_new_deadline;
                wait.as_mut().reset(new_deadline.clone());
            } else {break}
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