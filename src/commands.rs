use chrono::{DateTime, NaiveDateTime, Utc};
use poise::CreateReply;
use poise::serenity_prelude::{
    Colour, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage
};
use scraper::{Html, Selector};
use scraper::selectable::Selectable;
use crate::{Error, Context};

const NEXTSPACEFLIGHT_LINK: &'static str = "https://nextspaceflight.com/launches/";
const INTERACTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3600);

#[derive(Debug, Clone)]
struct FlightData {
    name: String,
    time: DateTime<Utc>,
    launch_site: String,
    details: String,
}

impl FlightData {
    fn formatted_time(&self) -> String {
        format!("<t:{}:F>", self.time.timestamp())
    }

    fn to_embed(&self, counter: usize) -> CreateEmbed {
        CreateEmbed::new()
            .footer(CreateEmbedFooter::new("Via NextSpaceflight"))
            .fields(vec![
                ("Time", self.formatted_time(), false),
                ("Launch Site", String::from(&self.launch_site), false),
            ])
            .title(format!("#{} | {}", counter, self.name.trim()))
            .url(format!("https://nextspaceflight.com{}", self.details))
            .color(Colour::new(0xFFFFFF))
    }
}

fn parse_time(time_str: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(time_str, "%a %b %d, %Y %H:%M %Z")
        .ok()
        .map(|t| DateTime::from_naive_utc_and_offset(t, Utc))
}

async fn fetch_launches() -> Result<Vec<FlightData>, Error> {
    let res = reqwest::get(NEXTSPACEFLIGHT_LINK).await?.text().await?;
    let document = Html::parse_document(&res);

    let mdl_card = Selector::parse(".mdl-card").unwrap();
    let header = Selector::parse("h5.header-style").unwrap();
    let launch_location = Selector::parse(".mdl-card__supporting-text").unwrap();
    let details_button = Selector::parse(".mdc-button").unwrap();

    document
        .select(&mdl_card)
        .filter_map(|launch| {
            let launch_data: Vec<_> = launch
                .select(&launch_location)
                .next()?
                .text()
                .collect();

            if launch_data.len() != 4 {
                return None;
            }

            let time = parse_time(launch_data[1])?;

            Some(FlightData {
                name: launch.select(&header).next()?.text().next()?.to_string(),
                launch_site: launch_data[3].to_string(),
                time,
                details: launch.select(&details_button).next()?.value().attr("href")?.to_string(),
            })
        })
        .map(Ok)
        .collect()
}

#[poise::command(slash_command)]
pub async fn fetch(ctx: Context<'_>) -> Result<(), Error> {
    let launches = fetch_launches().await?;

    let embed_pages: Vec<CreateEmbed> = launches
        .iter()
        .enumerate()
        .map(|(i, flight)| flight.to_embed(i + 1))
        .collect();

    let ctx_id = ctx.id();
    let prev_button_id = format!("{}previous", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    let initial_reply = {
        let components = CreateActionRow::Buttons(vec![
            CreateButton::new(&prev_button_id).label("Previous"),
            CreateButton::new(&next_button_id).label("Next")
        ]);

        CreateReply::default()
            .embed(embed_pages[0].clone())
            .components(vec![components])
    };

    ctx.send(initial_reply).await?;

    let mut page_num = 0;
    while let Some(press) = ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(INTERACTION_TIMEOUT)
        .await
    {
        let total = embed_pages.len();
        page_num = match press.data.custom_id.as_str() {
            id if id == next_button_id => (page_num + 1) % total,
            id if id == prev_button_id => page_num.checked_sub(1).unwrap_or(total - 1),
            _ => page_num,
        };

        press.create_response(
            ctx.serenity_context(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed_pages[page_num].clone())
            )
        ).await?;
    }

    Ok(())
}