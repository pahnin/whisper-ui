use iced::widget::{button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::widget::container;
use iced::{Element, Length};

use crate::app::model_status::ModelStatus;
use crate::app::Message;
use crate::inference::backend::model_manager::ModelInfo;

fn dark_panel_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.18,
                g: 0.18,
                b: 0.22,
                a: 1.0,
            })),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: Some(iced::Color::WHITE),
        }
    }
}

fn dark_overlay_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.05,
                g: 0.05,
                b: 0.08,
                a: 0.95,
            })),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: Some(iced::Color::WHITE),
        }
    }
}

fn build_settings_content<'a>(
    models: &'a [ModelInfo],
    selected_idx: usize,
    downloading_model: Option<usize>,
    model_status: &'a ModelStatus,
    language: &'a str,
    language_options: &'a [(String, String)],
    backend_ready: bool,
    language_search: &'a str,
) -> Column<'a, Message> {
    let model_list = build_model_list(models, selected_idx, downloading_model, model_status, backend_ready);
    let lang_section = build_lang_section(language, language_options, language_search);
    let status = build_status_text(model_status, backend_ready);
    let buttons = build_buttons(selected_idx, backend_ready);

    Column::new()
        .spacing(16)
        .push(Text::new("Settings").size(20))
        .push(
            Column::new()
                .spacing(4)
                .push(Text::new("Model").size(14))
                .push(model_list),
        )
        .push(lang_section)
        .push(status)
        .push(buttons)
        .padding(24)
        .width(500)
}

fn build_lang_section<'a>(
    language: &'a str,
    language_options: &'a [(String, String)],
    language_search: &'a str,
) -> Column<'a, Message> {
    let search_input = TextInput::new("Search languages...", language_search)
        .on_input(Message::LanguageSearch)
        .size(13)
        .padding([3, 8]);

    let lang_list = build_lang_list(language, language_options, language_search);

    Column::new()
        .spacing(4)
        .push(Text::new("Language").size(14))
        .push(search_input)
        .push(lang_list)
}

pub fn view<'a>(
    show: bool,
    models: &'a [ModelInfo],
    selected_idx: usize,
    language: &'a str,
    model_status: &'a ModelStatus,
    backend_ready: bool,
    downloading_model: Option<usize>,
    error_message: Option<&'a str>,
    language_options: &'a [(String, String)],
    language_search: &'a str,
) -> Option<Element<'a, Message>> {
    if !show {
        return None;
    }

    let panel_content = build_settings_content(models, selected_idx, downloading_model, model_status, language, language_options, backend_ready, language_search);
    let panel = Container::new(panel_content)
        .width(500)
        .height(Length::Shrink)
        .style(dark_panel_style())
        .padding(8);

    let overlay = |content: Container<'a, Message>| -> Element<'a, Message> {
        let inner = Container::new(content).padding(20).style(dark_overlay_style());
        Container::new(inner)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .into()
    };

    if let Some(err) = error_message {
        let error_row: Element<Message> = Row::new()
            .push(Text::new(format!("Error: {}", err)).color(iced::Color { r: 1.0, g: 0.4, b: 0.4, a: 1.0 }).size(12))
            .push(
                button(Text::new("Dismiss"))
                    .on_press(Message::ClearError),
            )
            .spacing(12)
            .padding(8)
            .into();
        let content = Container::new(
            Column::new()
                .spacing(16)
                .push(error_row)
                .push(panel)
                .align_x(iced::Alignment::Center)
        );
        Some(overlay(content))
    } else {
        let content = Container::new(
            Column::new()
                .push(panel)
                .align_x(iced::Alignment::Center)
        );
        Some(overlay(content))
    }
}

fn build_model_list<'a>(
    models: &'a [ModelInfo],
    selected_idx: usize,
    downloading_model: Option<usize>,
    model_status: &'a ModelStatus,
    backend_ready: bool,
) -> Element<'a, Message> {
    if models.is_empty() {
        return Container::new(
            Text::new("No models available. Download a model from the list below.").size(13)
        )
        .width(Length::Fill)
        .height(150)
        .into();
    }

    let list: Column<'a, Message> = models
        .iter()
        .enumerate()
        .fold(Column::new().spacing(2), |col, (i, model)| {
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

            let right_side: Element<'a, Message> = if downloaded && is_selected && !backend_ready && !is_downloading {
                button(Text::new("Load").size(12))
                    .on_press(Message::LoadModel(i))
                    .into()
            } else if downloaded && is_selected && backend_ready && !is_downloading {
                Text::new("\u{2713}").color(iced::Color { r: 0.3, g: 0.8, b: 0.3, a: 1.0 }).into()
            } else if downloaded && !is_selected && !is_downloading {
                Text::new("\u{2713}").color(iced::Color { r: 0.3, g: 0.8, b: 0.3, a: 1.0 }).into()
            } else if is_downloading {
                let pct = progress.unwrap_or(0.0);
                let filled = (pct / 10.0) as usize;
                let bar: String = "\u{2588}".repeat(filled.min(10))
                    + &"\u{2591}".repeat((10 - filled.min(10)).max(0));
                Text::new(format!(" {} {:.0}%", bar, pct)).into()
            } else {
                button(Text::new("Download").size(12))
                    .on_press(Message::DownloadModel(i))
                    .into()
            };

            let label_elem: Element<'a, Message> = button(Text::new(label).size(13))
                .on_press(Message::ModelSelected(i))
                .into();

            let row: Element<'a, Message> = if downloaded && is_selected && !backend_ready && !is_downloading {
                Row::new()
                    .push(label_elem)
                    .push(right_side)
                    .spacing(12)
                    .into()
            } else {
                Row::new()
                    .push(label_elem)
                    .push(right_side)
                    .spacing(12)
                    .into()
            };

            col.push(row)
        });

    Container::new(
        Scrollable::new(list)
            .width(Length::Fill)
            .height(150),
    )
    .width(Length::Fill)
    .height(150)
    .into()
}

fn build_lang_list<'a>(language: &'a str, language_options: &'a [(String, String)], search: &'a str) -> Element<'a, Message> {
    let search_lower = search.to_lowercase();
    let filtered: Vec<&(String, String)> = language_options
        .iter()
        .filter(|(code, full_name)| {
            if search.is_empty() {
                true
            } else {
                let code_match = code.to_lowercase().contains(&search_lower);
                let name_match = full_name.to_lowercase().contains(&search_lower);
                code_match || name_match
            }
        })
        .collect();

    if filtered.is_empty() && !search.is_empty() {
        Container::new(Text::new("No matching languages").size(13))
            .width(Length::Fill)
            .height(150)
            .into()
    } else {
        let list: Column<'a, Message> = filtered
            .iter()
            .fold(Column::new().spacing(2), |col, (code, full_name)| {
                let label = if code.as_str() == language {
                    format!("▸ {}", full_name)
                } else {
                    format!("  {}", full_name)
                };
                let item: Element<'a, Message> = button(Text::new(label))
                    .on_press(Message::LanguageChanged(code.clone()))
                    .into();
                col.push(item)
            });

        Container::new(
            Scrollable::new(list)
                .width(Length::Fill)
                .height(150),
        )
        .width(Length::Fill)
        .height(150)
        .into()
    }
}

fn build_status_text(model_status: &ModelStatus, backend_ready: bool) -> Element<'_, Message> {
    let text = match *model_status {
        ModelStatus::Downloading(pct) => format!("Downloading: {:.1}%", pct),
        ModelStatus::Ready if backend_ready => "Model loaded and ready".to_string(),
        ModelStatus::Ready => "Model downloaded — click Load Model to use".to_string(),
        ModelStatus::NotDownloaded => "Not downloaded — download a model to get started".to_string(),
        ModelStatus::Error(ref e) => format!("Error: {}", e),
    };
    Text::new(text).size(12).into()
}

fn build_buttons<'a>(selected_idx: usize, backend_ready: bool) -> Element<'a, Message> {
    let mut row = Row::new()
        .push(
            button(Text::new("Save"))
                .on_press(Message::SaveSettings),
        )
        .push(
            button(Text::new("Cancel"))
                .on_press(Message::HideSettings),
        )
        .spacing(16);

    if backend_ready {
        row = row.push(
            Text::new("\u{2713} Model Loaded").color(iced::Color { r: 0.3, g: 0.8, b: 0.3, a: 1.0 }).size(12),
        );
    } else {
        row = row.push(
            button(Text::new("Load Model"))
                .on_press(Message::LoadModel(selected_idx)),
        );
    }

    row.into()
}
