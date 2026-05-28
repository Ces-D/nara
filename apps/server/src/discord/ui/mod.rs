use brainiac_core::database::models::{PracticeItem, PracticeItemAnswer};
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use titans_tower::{COLOR_ACCENT_PRIMARY, COLOR_ACCENT_SECONDARY};

pub mod paginate;

pub fn practice_item_embed(item: &PracticeItem) -> CreateEmbed {
    CreateEmbed::new()
        .title("Question")
        .description(format!("## {}", item.front))
        .color(COLOR_ACCENT_PRIMARY)
        .footer(CreateEmbedFooter::new(format!("ID: {}", item.id)))
}

pub fn answer_embed(item: &PracticeItemAnswer) -> CreateEmbed {
    CreateEmbed::new()
        .title("Answer")
        .description(format!("## {}", item.back))
        .color(COLOR_ACCENT_SECONDARY)
        .footer(CreateEmbedFooter::new(format!("ID: {}", item.id)))
}
