use iced::widget::{button, Container, Row, Text};
use iced::Element;

use crate::app::Message;

pub fn view<'a>(
    show: bool,
    _models: &'a [String],
    selected: usize,
) -> Option<Element<'a, Message>> {
    if !show {
        return None;
    }

    let modal = Container::new(
        iced::widget::column![
            Text::new("Settings").size(20),
            Text::new(format!("Selected model index: {}", selected)),
            Text::new("Model download and language settings would go here."),
            Row::new()
                .push(
                    button(Text::new("Save"))
                        .on_press(Message::SaveSettings),
                )
                .push(button(Text::new("Cancel")).on_press(Message::HideSettings)),
        ]
        .spacing(16)
        .padding(24)
        .width(400),
    );

    Some(Container::new(modal).into())
}
