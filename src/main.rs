use std::env;

use hemoglobin::cards::Card;
use regex::Regex;
use serde::Deserialize;
use serenity::{
    all::{Context, EventHandler, GatewayIntents, Message, Ready},
    async_trait, Client,
};

struct Handler;

#[derive(Deserialize, PartialEq)]
#[serde(tag = "type")]
enum QueryResult {
    CardList {
        query_text: String,
        content: Vec<Card>,
    },
    Error {
        message: String,
    },
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        for mtch in Regex::new(r"\{\{(.*)\}\}")
            .unwrap()
            .captures_iter(&msg.content)
            .filter_map(|x| x.get(1))
        {
            let mut mtch = mtch.as_str().trim().to_owned();
            while mtch.contains("  ") {
                mtch = mtch.replace("  ", " ");
            }
            let mtch = mtch;
            let api_result = reqwest::get(format!(
                "http://hemolymph.ampersandia.net/api/search?query=n:\"{}\"",
                mtch.to_lowercase().replace(' ', "_")
            ))
            .await;

            println!("{}", mtch.to_lowercase().replace(' ', "_"));

            match api_result {
                Ok(result) => match result.json::<QueryResult>().await {
                    Ok(QueryResult::CardList {
                        query_text: _,
                        content,
                    }) => {
                        if let Some(card) = content.first() {
                            if let Err(why) = msg
                                .channel_id
                                .say(
                                    &ctx.http,
                                    format!("http://hemolymph.ampersandia.net/card/{}", card.id),
                                )
                                .await
                            {
                                eprintln!("Error sending message: {why:?}");
                            }
                        } else if let Err(why) = msg
                            .channel_id
                            .say(&ctx.http, "Couldn't find card".to_string())
                            .await
                        {
                            eprintln!("Error sending message: {why:?}");
                        }
                    }
                    Ok(QueryResult::Error { message: _ }) => {
                        if let Err(why) = msg
                            .channel_id
                            .say(&ctx.http, "Couldn't find card".to_string())
                            .await
                        {
                            eprintln!("Error sending message: {why:?}");
                        }
                    }
                    Err(error) => {
                        if let Err(why) = msg
                            .channel_id
                            .say(&ctx.http, format!("Couldn't find card: {error}"))
                            .await
                        {
                            eprintln!("Error sending message: {why:?}");
                        }
                    }
                },
                Err(error) => {
                    if let Err(why) = msg
                        .channel_id
                        .say(&ctx.http, format!("Couldn't reach server: {error}"))
                        .await
                    {
                        eprintln!("Error sending message: {why:?}");
                    }
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("TOKEN").expect("Couldn't find a token in the env vars");

    let intent = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intent)
        .event_handler(Handler)
        .await
        .expect("Could not create client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why}");
    }
}
