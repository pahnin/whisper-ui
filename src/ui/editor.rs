use iced::widget::{button, Column, Container, Scrollable, Text, TextInput};
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
    _append_mode: bool,
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
                .height(150)
                .width(Length::Fill),
        );

    let edit_title = Text::new("Edit transcript").size(12);

    let edit_input = TextInput::new("Edit transcript...", temp_content)
        .on_input(Message::ContentChangedTemp)
        .on_submit(Message::CommitContent)
        .width(Length::Fill)
        .size(14);

    let action_buttons = Column::new()
        .spacing(4)
        .push(Text::new("Transcript ready to append").size(12))
        .push(
            Column::new()
                .push(
                    button(Text::new("Append to Document"))
                        .on_press(Message::AppendTranscript)
                )
                .push(
                    button(Text::new("Discard"))
                        .on_press(Message::ContentChangedTemp(String::new()))
                )
                .spacing(4)
        );

    let bottom_section = Column::new()
        .spacing(8)
        .push(edit_title)
        .push(edit_input)
        .push(action_buttons);

    let main_content = Column::new()
        .push(editor)
        .push(rule::horizontal(1))
        .push(bottom_section)
        .height(Length::Fill);

    Container::new(main_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(editor_style())
        .into()
}
