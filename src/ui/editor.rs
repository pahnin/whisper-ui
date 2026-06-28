use iced::widget::{Column, Container, Row, Scrollable, Text, rule};
use iced::widget::container;
use iced::{Element, Length};

use crate::app::Message;
use crate::document::{Document, TranscriptLine};

pub fn view<'a>(active_doc: Option<&'a Document>) -> Element<'a, Message> {
    let lines: &'a [TranscriptLine] = active_doc
        .map(|doc| doc.transcript_lines.as_slice())
        .unwrap_or_default();

    let editor = Column::new()
        .spacing(8)
        .padding(16)
        .height(Length::Fill)
        .width(Length::Fill)
        .push(Text::new("Transcript").size(18))
        .push(rule::horizontal(1))
        .push(build_scrollable_content(lines));

    Container::new(editor)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(editor_style)
        .into()
}

fn build_scrollable_content<'a>(lines: &'a [TranscriptLine]) -> Element<'a, Message> {
    Scrollable::new(
        Row::new()
            .push(build_timestamps_column(lines))
            .push(build_text_column(lines))
            .spacing(0)
            .height(Length::Fill),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}

fn build_timestamps_column<'a>(lines: &'a [TranscriptLine]) -> Column<'a, Message> {
    let mut col = Column::new()
        .width(70)
        .spacing(0);
    for line in lines {
        col = col.push(
            Text::new(&line.timestamp)
                .size(12)
                .font(iced::font::Font {
                    family: iced::font::Family::Monospace,
                    ..iced::font::Font::default()
                }),
        );
    }
    if lines.is_empty() {
        col = col.push(
            Text::new(" ")
                .size(12)
                .font(iced::font::Font {
                    family: iced::font::Family::Monospace,
                    ..iced::font::Font::default()
                }),
        );
    }
    col
}

fn build_text_column<'a>(lines: &'a [TranscriptLine]) -> Column<'a, Message> {
    let mut col = Column::new()
        .width(Length::Fill)
        .spacing(0);
    for line in lines {
        col = col.push(Text::new(&line.text).size(12));
    }
    if lines.is_empty() {
        col = col.push(
            Text::new("No transcript yet. Start recording to capture text.")
                .size(12)
                .font(iced::font::Font {
                    family: iced::font::Family::Monospace,
                    ..iced::font::Font::default()
                }),
        );
    }
    col
}

fn editor_style(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(iced::Color {
            r: 0.09,
            g: 0.09,
            b: 0.11,
            a: 1.0,
        })),
        text_color: Some(iced::Color::WHITE),
        ..container::Style::default()
    }
}
