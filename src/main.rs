use std::env;
use std::fmt::Write;

use hemoglobin::cards::Card;
use regex::Regex;
use serde::Deserialize;
use serenity::{
    all::{
        CacheHttp, ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateMessage, EventHandler,
        GatewayIntents, Message, Ready,
    },
    async_trait,
    futures::TryFutureExt,
    Client,
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

// .or_else(|_| reqwest::get(format!("http://hemolymph.net/api/search?query={}", mtch)))
#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        for mtch in Regex::new(r"\{\{([^!]*?)\}\}")
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
                "http://hemolymph.net/api/search?query=n:\"{}\"",
                mtch.to_lowercase()
            ))
            .and_then(|x| x.json::<QueryResult>())
            .await;

            match api_result {
                Ok(QueryResult::CardList {
                    query_text: _,
                    content,
                }) => {
                    if let Some(card) = content.first() {
                        message_for_card(&msg.channel_id, &ctx.http, card).await;
                    } else {
                        send_and_report(
                            &ctx.http,
                            "Couldn't find a matching card.",
                            &msg.channel_id,
                        )
                        .await;
                    }
                }
                Ok(QueryResult::Error { message }) => {
                    send_and_report(
                        &ctx.http,
                        format!("Couldn't parse search: {message}"),
                        &msg.channel_id,
                    )
                    .await;
                }
                Err(error) => {
                    send_and_report(
                        &ctx.http,
                        format!("Couldn't perform search: {error}"),
                        &msg.channel_id,
                    )
                    .await;
                }
            }
        }
        for mtch in Regex::new(r"\{\{!+?(.*?)\}\}")
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
                "http://hemolymph.net/api/search?query={}",
                mtch.to_lowercase()
            ))
            .and_then(|x| x.json::<QueryResult>())
            .await;

            match api_result {
                Ok(QueryResult::CardList {
                    query_text: _,
                    content,
                }) => {
                    if let Some(card) = content.first() {
                        message_for_card(&msg.channel_id, &ctx.http, card).await;
                    } else {
                        send_and_report(
                            &ctx.http,
                            "Couldn't find a matching card.",
                            &msg.channel_id,
                        )
                        .await;
                    }
                }
                Ok(QueryResult::Error { message }) => {
                    send_and_report(
                        &ctx.http,
                        format!("Couldn't parse search: {message}"),
                        &msg.channel_id,
                    )
                    .await;
                }
                Err(error) => {
                    send_and_report(
                        &ctx.http,
                        format!("Couldn't perform search: {error}"),
                        &msg.channel_id,
                    )
                    .await;
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

async fn send_and_report(
    cache_http: impl CacheHttp,
    message: impl Into<String>,
    channel: &ChannelId,
) {
    match channel.say(cache_http, message).await {
        Ok(_) => (),
        Err(x) => eprintln!("Couldn't send message: {x}"),
    }
}

async fn message_for_card(channel: &ChannelId, http: impl CacheHttp, card: &Card) {
    let footer = CreateEmbedFooter::new(
        get_card_footer_text(card).unwrap_or("Failed to generate cost-typeline".to_owned()),
    );
    let embed = CreateEmbed::new()
        .title(card.name.clone())
        .image(format!(
            "https://file.garden/ZJSEzoaUL3bz8vYK/bloodlesscards/{}.png",
            card.get_image()
        ))
        .footer(footer)
        .description(get_card_embed_text(card));

    let msg = CreateMessage::new()
        .embed(embed)
        .content(format!("http://hemolymph.ampersandia.net/card/{}", card.id));

    match channel.send_message(http, msg).await {
        Ok(_) => (),
        Err(x) => eprintln!("Couldn't send card message: {x}"),
    }
}

fn get_card_embed_text(card: &Card) -> String {
    card.description.clone()
}

fn get_card_footer_text(card: &Card) -> Result<String, core::fmt::Error> {
    let mut string = String::new();
    if card.r#type.to_lowercase().contains("flask") {
        write!(&mut string, "{}", get_ascii_titlecase(&card.r#type))?;
    } else {
        write!(
            &mut string,
            "{} :: {} Blood",
            get_ascii_titlecase(&card.r#type),
            card.cost
        )?;
    }

    if !card.r#type.to_lowercase().contains("command") {
        write!(
            &mut string,
            " :: {}/{}/{}",
            card.health, card.defense, card.power
        )?;
    }
    Ok(string)
}

fn get_ascii_titlecase(s: &str) -> String {
    let mut b = s.to_string();
    if let Some(r) = b.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    b
}
