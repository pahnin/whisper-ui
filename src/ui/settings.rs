use iced::widget::{button, Column, Container, Row, Scrollable, Text};
use iced::{Element, Length};

use crate::app::model_status::ModelStatus;
use crate::app::Message;
use crate::inference::backend::model_manager::ModelInfo;
use crate::inference::backend::model_manager::ModelManager;

pub fn view<'a>(
    show: bool,
    models: &'a [ModelInfo],
    selected_idx: usize,
    language: &'a str,
    model_status: &'a ModelStatus,
    _backend_ready: bool,
    downloading_model: Option<usize>,
) -> Option<Element<'a, Message>> {
    if !show {
        return None;
    }

    let model_list = build_model_list(models, selected_idx, downloading_model, model_status);
    let lang_list = build_lang_list(language);
    let status = build_status_text(model_status);
    let buttons = build_buttons();

    let content = Column::new()
        .spacing(16)
        .push(Text::new("Settings").size(20))
        .push(
            Column::new()
                .spacing(4)
                .push(Text::new("Model").size(14))
                .push(model_list),
        )
        .push(
            Column::new()
                .spacing(4)
                .push(Text::new("Language").size(14))
                .push(lang_list),
        )
        .push(status)
        .push(buttons)
        .padding(24)
        .width(500);

    Some(Container::new(content).into())
}

fn build_model_list<'a>(
    models: &'a [ModelInfo],
    selected_idx: usize,
    downloading_model: Option<usize>,
    model_status: &'a ModelStatus,
) -> Element<'a, Message> {
    let items: Vec<Element<Message>> = models
        .iter()
        .enumerate()
        .map(|(i, model)| {
            let is_selected = i == selected_idx;
            let downloaded = model.downloaded;
            let is_downloading = downloading_model == Some(i);
            let progress = if is_downloading {
                match *model_status {
                    ModelStatus::Downloading(pct) => Some(pct),
                    _ => None,
                }
            } else {
                None
            };

            let label = if is_selected {
                format!("▸ {} ({:.1}MB)", model.name, model.size_bytes as f64 / 1_000_000.0)
            } else {
                format!("  {} ({:.1}MB)", model.name, model.size_bytes as f64 / 1_000_000.0)
            };

            let right_side: Element<Message> = if downloaded {
                Text::new("[\u{2713}]").into()
            } else if is_downloading {
                let pct = progress.unwrap_or(0.0);
                let filled = (pct / 10.0) as usize;
                let bar: String = "\u{2588}".repeat(filled.min(10))
                    + &"\u{2591}".repeat((10 - filled.min(10)).max(0));
                Text::new(format!(" {} {:.0}%", bar, pct)).into()
            } else {
                button(Text::new("Download"))
                    .on_press(Message::DownloadModel(i))
                    .into()
            };

            let row = Row::new()
                .push(
                    button(Text::new(label)).on_press(Message::ModelSelected(i))
                )
                .push(right_side)
                .spacing(8)
                .into();

            row
        })
        .collect();

    Container::new(
        Scrollable::new(Column::new().extend(items))
            .width(Length::Fill)
            .height(150),
    )
    .into()
}

fn build_lang_list<'a>(language: &'a str) -> Element<'a, Message> {
    let options = ModelManager::language_options();
    let items: Vec<_> = options
        .iter()
        .map(|(code, full_name)| {
            let label = if code == language {
                format!("▸ {}", full_name)
            } else {
                format!("  {}", full_name)
            };
            button(Text::new(label))
                .on_press(Message::LanguageChanged(code.clone()))
                .into()
        })
        .collect();

    Container::new(
        Scrollable::new(Column::new().extend(items))
            .width(Length::Fill)
            .height(150),
    )
    .into()
}

fn build_status_text(model_status: &ModelStatus) -> Element<'_, Message> {
    let text = match *model_status {
        ModelStatus::Downloading(pct) => format!("Downloading: {:.1}%", pct),
        ModelStatus::Ready => "Model loaded".to_string(),
        ModelStatus::NotDownloaded => "No model selected".to_string(),
        ModelStatus::Error(ref e) => format!("Error: {}", e),
    };
    Text::new(text).size(12).into()
}

fn build_buttons<'a>() -> Element<'a, Message> {
    Row::new()
        .push(
            button(Text::new("Save"))
                .on_press(Message::SaveSettings),
        )
        .push(
            button(Text::new("Cancel"))
                .on_press(Message::HideSettings),
        )
        .spacing(16)
        .into()
}
