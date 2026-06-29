use iced::widget::{button, Column, Container, Row, Scrollable, Text, TextInput};
use iced::widget::button::Style as ButtonStyle;
use iced::widget::container;
use iced::widget::container::Style as ContainerStyle;
use iced::{Alignment, Color, Element, Length};

use crate::app::model_status::ModelStatus;
use crate::app::Message;
use crate::inference::backend::model_manager::ModelInfo;

const PANEL_BG: Color = Color {
    r: 0.14,
    g: 0.14,
    b: 0.17,
    a: 1.0,
};

const PANEL_BORDER: Color = Color {
    r: 0.18,
    g: 0.18,
    b: 0.22,
    a: 0.5,
};

const TEXT_PRIMARY: Color = Color {
    r: 0.88,
    g: 0.88,
    b: 0.92,
    a: 1.0,
};

const TEXT_MUTED: Color = Color {
    r: 0.42,
    g: 0.42,
    b: 0.48,
    a: 1.0,
};

const TEXT_DIM: Color = Color {
    r: 0.30,
    g: 0.30,
    b: 0.35,
    a: 1.0,
};

const ACCENT: Color = Color {
    r: 0.55,
    g: 0.45,
    b: 0.75,
    a: 1.0,
};

const INPUT_BORDER: Color = Color {
    r: 0.20,
    g: 0.20,
    b: 0.25,
    a: 1.0,
};

const CHECKMARK: Color = Color {
    r: 0.30,
    g: 0.80,
    b: 0.30,
    a: 1.0,
};

fn panel_style() -> impl Fn(&iced::Theme) -> ContainerStyle {
    |_| ContainerStyle {
        background: Some(iced::Background::Color(PANEL_BG)),
        border: iced::Border {
            color: PANEL_BORDER,
            radius: 6.0.into(),
            width: 1.0,
        },
        text_color: Some(TEXT_PRIMARY),
        ..ContainerStyle::default()
    }
}

fn overlay_style() -> impl Fn(&iced::Theme) -> ContainerStyle {
    |_| ContainerStyle {
        background: Some(iced::Background::Color(Color {
            r: 0.05,
            g: 0.05,
            b: 0.08,
            a: 0.95,
        })),
        ..ContainerStyle::default()
    }
}

fn section_heading_style() -> impl Fn(&iced::Theme) -> ContainerStyle {
    |_| ContainerStyle {
        background: Some(iced::Background::Color(Color {
            r: 0.12,
            g: 0.12,
            b: 0.15,
            a: 1.0,
        })),
        border: iced::Border {
            color: ACCENT,
            radius: 0.0.into(),
            width: 1.5,
        },
        ..ContainerStyle::default()
    }
}

fn ghost_button_style(
    text_color: Color,
    border_color: Color,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> ButtonStyle {
    move |_theme, _status| ButtonStyle {
        text_color,
        border: iced::Border {
            color: border_color,
            radius: 4.0.into(),
            width: 1.0,
        },
        ..ButtonStyle::default()
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

    let title_row: Element<'a, Message> = Row::new()
        .push(Text::new("Settings").size(18))
        .align_y(Alignment::Center)
        .height(32)
        .into();

    Column::new()
        .spacing(20)
        .push(title_row)
        .push(
            Column::new()
                .spacing(10)
                .push(
                    Container::new(Text::new("Model").size(12))
                        .padding([6, 10])
                        .style(section_heading_style()),
                )
                .push(model_list),
        )
        .push(
            Column::new()
                .spacing(10)
                .push(
                    Container::new(Text::new("Language").size(12))
                        .padding([6, 10])
                        .style(section_heading_style()),
                )
                .push(lang_section),
        )
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
        .padding([8, 12])
        .width(Length::Fill);

    let lang_list = build_lang_list(language, language_options, language_search);

    Column::new()
        .spacing(10)
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
        .style(panel_style())
        .padding(0);

    let overlay = |content: Container<'a, Message>| -> Element<'a, Message> {
        let inner = Container::new(content).padding(20).style(overlay_style());
        Container::new(inner)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    };

    if let Some(err) = error_message {
        let error_row: Element<Message> = Row::new()
            .push(Text::new(format!("Error: {}", err)).color(Color { r: 0.85, g: 0.35, b: 0.35, a: 1.0 }).size(12))
            .push(
                button(Text::new("Dismiss").size(12))
                    .on_press(Message::ClearError)
                    .style(ghost_button_style(TEXT_PRIMARY, INPUT_BORDER)),
            )
            .spacing(16)
            .padding(10)
            .into();
        let content = Container::new(
            Column::new()
                .spacing(16)
                .push(error_row)
                .push(panel)
                .align_x(iced::Alignment::Center),
        );
        Some(overlay(content))
    } else {
        let content = Container::new(
            Column::new()
                .push(panel)
                .align_x(iced::Alignment::Center),
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
            Text::new("No models available. Download a model from the list below.")
                .size(13)
                .color(TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(150)
        .into();
    }

    let list: Column<'a, Message> = models
        .iter()
        .enumerate()
        .fold(Column::new().spacing(4), |col, (i, model)| {
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

            let label_color = if is_selected { TEXT_PRIMARY } else { TEXT_MUTED };

            let right_side: Element<'a, Message> = if downloaded && is_selected && !backend_ready && !is_downloading {
                button(Text::new("Load").size(12))
                    .on_press(Message::LoadModel(i))
                    .style(ghost_button_style(ACCENT, ACCENT))
                    .into()
            } else if downloaded && is_selected && backend_ready && !is_downloading {
                Text::new("✓").color(CHECKMARK).into()
            } else if downloaded && !is_selected && !is_downloading {
                Text::new("✓").color(CHECKMARK).into()
            } else if is_downloading {
                let pct = progress.unwrap_or(0.0);
                build_minimal_progress_bar(pct)
            } else {
                button(Text::new("Download").size(12))
                    .on_press(Message::DownloadModel(i))
                    .style(ghost_button_style(TEXT_MUTED, INPUT_BORDER))
                    .into()
            };

            let label_elem: Element<'a, Message> = button(Text::new(label).size(13).color(label_color))
                .on_press(Message::ModelSelected(i))
                .style(ghost_button_style(label_color, Color::TRANSPARENT))
                .into();

            let row: Element<'a, Message> = Row::new()
                .push(label_elem)
                .push(right_side)
                .spacing(12)
                .align_y(Alignment::Center)
                .into();

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

fn build_minimal_progress_bar(pct: f32) -> Element<'static, Message> {
    let width = 100.0f32;
    let filled = (pct / 100.0) * width;

    let filled_bar = Container::new(
        Container::new(iced::widget::space())
            .width(Length::Fixed(filled.max(4.0)))
            .height(Length::Fixed(2.0))
            .style(|_| container::Style {
                background: Some(iced::Background::Color(ACCENT)),
                ..container::Style::default()
            }),
    )
    .height(Length::Fixed(2.0))
    .width(Length::Fixed(width))
    .style(|_| container::Style {
        background: Some(iced::Background::Color(INPUT_BORDER)),
        ..container::Style::default()
    });

    let pct_text = Text::new(format!("{:.0}%", pct)).size(10).color(TEXT_MUTED);

    Row::new()
        .push(filled_bar)
        .push(pct_text)
        .spacing(8)
        .align_y(Alignment::Center)
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
        Container::new(Text::new("No matching languages").size(13).color(TEXT_MUTED))
            .width(Length::Fill)
            .height(150)
            .into()
    } else {
        let list: Column<'a, Message> = filtered
            .iter()
            .fold(Column::new().spacing(4), |col, (code, full_name)| {
                let is_selected = code.as_str() == language;
                let label = if is_selected {
                    format!("▸ {}", full_name)
                } else {
                    format!("  {}", full_name)
                };
                let item_color = if is_selected { TEXT_PRIMARY } else { TEXT_MUTED };
                let item: Element<'a, Message> = button(Text::new(label).size(13).color(item_color))
                    .on_press(Message::LanguageChanged(code.clone()))
                    .style(ghost_button_style(item_color, Color::TRANSPARENT))
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
    let (text, color) = match *model_status {
        ModelStatus::Downloading(pct) => (format!("Downloading: {:.1}%", pct), TEXT_MUTED),
        ModelStatus::Ready if backend_ready => ("Model loaded and ready".to_string(), CHECKMARK),
        ModelStatus::Ready => ("Model downloaded — click Load Model to use".to_string(), TEXT_MUTED),
        ModelStatus::NotDownloaded => ("Not downloaded — download a model to get started".to_string(), TEXT_DIM),
        ModelStatus::Error(ref e) => (format!("Error: {}", e), Color { r: 0.85, g: 0.35, b: 0.35, a: 1.0 }),
    };
    Text::new(text).size(12).color(color).into()
}

fn build_buttons<'a>(selected_idx: usize, backend_ready: bool) -> Element<'a, Message> {
    let mut row = Row::new()
        .push(
            button(Text::new("Save").size(12))
                .on_press(Message::SaveSettings)
                .style(ghost_button_style(TEXT_PRIMARY, ACCENT)),
        )
        .push(
            button(Text::new("Cancel").size(12))
                .on_press(Message::HideSettings)
                .style(ghost_button_style(TEXT_MUTED, INPUT_BORDER)),
        )
        .spacing(12)
        .align_y(Alignment::Center);

    if backend_ready {
        row = row.push(
            Text::new("✓ Model Loaded").size(12).color(CHECKMARK),
        );
    } else {
        row = row.push(
            button(Text::new("Load Model").size(12))
                .on_press(Message::LoadModel(selected_idx))
                .style(ghost_button_style(ACCENT, ACCENT)),
        );
    }

    row.into()
}
