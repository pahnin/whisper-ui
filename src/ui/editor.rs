use iced::widget::{Column, Container, Scrollable, Text, TextInput};
use iced::{Length, Element};

use crate::document::Document;
use crate::app::Message;

pub fn view<'a>(
    active_doc: Option<&'a Document>,
    temp_content: &'a str,
) -> Element<'a, Message> {
    let title = if let Some(doc) = active_doc {
        doc.title.clone()
    } else {
        "New Document".to_string()
    };

    let editor = Column::new()
        .spacing(8)
        .padding(16)
        .height(Length::Fill)
        .push(Text::new("Transcript").size(18))
        .push(
            Scrollable::new(Text::new(format!("# {}\n\n{}", title, temp_content)))
                .height(Length::Fill),
        )
        .push(
            TextInput::new("Edit transcript...", temp_content)
                .on_input(Message::ContentChangedTemp)
                .on_submit(Message::CommitContent)
                .width(Length::Fill)
                .size(14),
        );

    Container::new(editor)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
