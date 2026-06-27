use iced::widget::{button, Container, Row, Text};
use iced::Element;

use crate::app::Message;

pub fn view<'a>(
    show: bool,
    models: &'a [String],
    selected: usize,
    language: &'a str,
) -> Option<Element<'a, Message>> {
    if !show {
        return None;
    }

    let _model_list: Vec<_> = models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if i == selected {
                format!("▸ {}", m)
            } else {
                format!("  {}", m)
            }
        })
        .collect();

    let modal = Container::new(
        iced::widget::column![
            Text::new("Settings").size(20),
            Text::new("Model Selection").size(16),
            Text::new(format!("Selected: {}", models.get(selected).unwrap_or(&"None".to_string()))),
            Text::new(format!("Language: {}", language)),
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
