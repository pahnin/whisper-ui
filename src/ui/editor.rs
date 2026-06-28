use iced::widget::{Column, Container, Scrollable, Text};
use iced::widget::container;
use iced::widget::rule;
use iced::{Element, Length};

use crate::document::Document;
use crate::app::Message;

fn editor_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.09,
                g: 0.09,
                b: 0.11,
                a: 1.0,
            })),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: Some(iced::Color::WHITE),
        }
    }
}

pub fn view<'a>(
    active_doc: Option<&'a Document>,
    temp_content: &'a str,
) -> Element<'a, Message> {
    let title = active_doc.map(|doc| doc.title.clone()).unwrap_or_else(|| "Untitled".to_string());

    let editor = Column::new()
        .spacing(8)
        .padding(16)
        .height(Length::Fill)
        .push(Text::new("Transcript").size(18))
        .push(rule::horizontal(1))
        .push(
            Scrollable::new({
                let combined = if let Some(doc) = active_doc {
                    format!("# {}\n\n{}", doc.title, doc.content) + temp_content
                } else {
                    format!("# {}\n\n{}", title, temp_content)
                };
                Text::new(combined)
            })
                .height(Length::Fill)
                .width(Length::Fill),
        );

    let main_content = Column::new()
        .push(editor)
        .height(Length::Fill);

    Container::new(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(editor_style())
        .into()
}
