
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

use super::crypto::CryptoHandler;

pub fn test1() {
    if &env::args().collect::<Vec<String>>()[2] == "0" {start_bot(true, Some(CryptoHandler::new()))}; // pc
    if &env::args().collect::<Vec<String>>()[2] == "1" {start_bot(false, None)}; // phone
}

#[group]
#[commands(ping)]
struct General;

/// handler for the main program. ie interface between user, discord, crypto
struct Handler {
    token: String,
    receiver: bool,
    crypto_handler: Option<CryptoHandler>,
}

impl Handler {
    // TODO: use this to increase the "now" variable before/after every interaction. use internal mutability
    pub fn wait_from_now(&self) {

    }

    pub async fn got_public_key(&self, strr: String, ctx: Context, message: Message) {
        let crypto_handler = CryptoHandler::new_from_public_key_pem(strr);
        loop {
            let pass = rpassword::prompt_password_stdout("enter password or something: ").unwrap();
            let encrypted_password = crypto_handler.encrypt(pass);
            message.channel_id.say(&ctx, format!("encrypted: {}", encrypted_password)).await.unwrap();
        }
        
    }
    
    pub fn got_encrypted(&self, encrypted: String) {
        // loop for 60 seconds in async looking for encrypted and then gracefully shut down
        //    idk where/how to do this
        let password = self.crypto_handler.as_ref().unwrap().decrypt(encrypted);
        dbg!(&password);
        #[cfg(feature = "enigo")]
        use enigo::{self, KeyboardControllable};
        #[cfg(feature = "enigo")]
        enigo::Enigo::new().key_sequence_parse(&password);
    }
}

#[async_trait]
impl EventHandler for Handler {
    // if receiver, send the private key
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        if !self.receiver {return}
        // for (_, channel) in ctx.cache.channels.clone().read() {
        //     if channel.name == "yankpass" {

        //     }
        // }
        let mut yank_channel: Option<GuildChannel> = None;
        // let http = Http::new_with_token(&self.token);
        'loup: for guild_id in guilds {
            if let Some(channel_map) = ctx.cache.guild_channels(guild_id).await {
                for (_, channel) in channel_map {
                    if channel.name == "yankpass" {
                        yank_channel = Some(channel.clone());
                        break 'loup
                    }
                }
            // if let Ok(channels) = http.get_channels(guild_id.0).await {
            //     for channel in channels {
            //         if channel.name == "yankpass" {
            //             yank_channel = Some(channel.clone());
            //             break 'loup
            //         }
            //     }
            }
        }
        if let Some(channel) = yank_channel {
            channel.say(&ctx, self.crypto_handler.as_ref().unwrap().public_key()).await.unwrap();
        } 
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        if self.receiver {
            let start = "encrypted: ";
            if new_message.author.bot && new_message.content.starts_with(start) {
                self.got_encrypted(new_message.content[start.len()..].into());
            }
        } else {
            if new_message.author.bot && new_message.content.starts_with("-----BEGIN PUBLIC KEY-----") {
                self.got_public_key(new_message.content.clone(), ctx, new_message).await;
            }
        }
    }

}

#[tokio::main]
async fn start_bot(receiver: bool, crypto_handler: Option<CryptoHandler>) {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    // let token = env::var("DISCORD_TOKEN").expect("token");
    let token = &env::args().collect::<Vec<String>>()[1];
    let handler = Handler {token: token.to_owned(), receiver, crypto_handler};
    let mut client = Client::builder(token)
        .event_handler(handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    let bot = client.start();
    let wait = tokio::time::sleep(std::time::Duration::from_secs(128));
    tokio::pin!(bot);
    tokio::pin!(wait);
    loop {
        tokio::select! {
            Err(why) = &mut bot => {
                println!("An error occurred while running the client: {:?}", why);
            },
            _ = &mut wait => panic!("an intensional panic to shut down program"),
        }
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, &msg.content).await?;
    msg.channel_id.say(ctx, &msg.content).await?;
    Ok(())
}