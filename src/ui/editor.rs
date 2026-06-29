use iced::widget::{Column, Container, Row, Scrollable, Space, Text};
use iced::widget::container;
use iced::{Color, Element, Length};

use crate::app::Message;
use crate::document::{Document, TranscriptLine};

fn editor_bg() -> Color {
    Color { r: 0.07, g: 0.07, b: 0.09, a: 1.0 }
}

fn editor_border() -> Color {
    Color { r: 0.18, g: 0.18, b: 0.22, a: 0.3 }
}

fn text_primary() -> Color {
    Color { r: 0.88, g: 0.88, b: 0.92, a: 1.0 }
}

fn text_muted() -> Color {
    Color { r: 0.42, g: 0.42, b: 0.48, a: 1.0 }
}

fn text_dim() -> Color {
    Color { r: 0.30, g: 0.30, b: 0.35, a: 1.0 }
}

fn heading() -> Color {
    Color { r: 0.65, g: 0.60, b: 0.72, a: 1.0 }
}

fn mono_font() -> iced::font::Font {
    iced::font::Font {
        family: iced::font::Family::Monospace,
        ..iced::font::Font::default()
    }
}

pub fn view<'a>(active_doc: Option<&'a Document>) -> Element<'a, Message> {
    let lines: &'a [TranscriptLine] = active_doc
        .map(|doc| doc.transcript_lines.as_slice())
        .unwrap_or_default();

    let editor = Column::new()
        .spacing(16)
        .padding(20)
        .height(Length::Fill)
        .width(Length::Fill)
        .push(
            Column::new()
                .spacing(4)
                .push(
                    Text::new("Transcript")
                        .size(14)
                        .color(heading()),
                )
                .push(
                    Text::new(active_doc.map(|d| d.title.as_str()).unwrap_or("Untitled"))
                        .size(12)
                        .color(text_muted()),
                ),
        )
        .push(build_scrollable_content(lines));

    Container::new(editor)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(editor_style)
        .into()
}

fn build_scrollable_content<'a>(lines: &'a [TranscriptLine]) -> Element<'a, Message> {
    if lines.is_empty() {
        let placeholder = Text::new("No transcript yet. Start recording to capture text.")
            .size(12)
            .color(text_muted());
        let placeholder_col = Column::new()
            .push(placeholder)
            .align_x(iced::Alignment::Center)
            .spacing(8)
            .width(Length::Fill);
        return Scrollable::new(placeholder_col)
            .height(Length::Fill)
            .width(Length::Fill)
            .into();
    }

    let mut line_rows = Column::new();
    for line in lines {
        let timestamp = Text::new(&line.timestamp)
            .size(11)
            .color(text_dim())
            .font(mono_font());

        let content = Text::new(&line.text)
            .size(13)
            .color(text_primary());

        let line_row = Row::new()
            .push(timestamp)
            .push(Space::new().width(Length::Fixed(16.0)))
            .push(content)
            .align_y(iced::Alignment::Center);

        line_rows = line_rows.push(line_row);
    }

    Scrollable::new(line_rows)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn editor_style(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(editor_bg())),
        border: iced::Border::default().rounded(3).color(editor_border()),
        text_color: Some(text_primary()),
        ..container::Style::default()
    }
}
