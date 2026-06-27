use iced::widget::{button, Container, Row, Text};
use iced::Length;

use crate::app::Message;

pub fn view<'a>(
    is_recording: bool,
    is_paused: bool,
    audio_level: f32,
) -> iced::Element<'a, Message> {
    let rec_btn = if is_recording && !is_paused {
        button(Text::new("Stop"))
            .on_press(Message::StopRecord)
    } else if is_paused {
        button(Text::new("Resume"))
            .on_press(Message::ResumeRecord)
    } else {
        button(Text::new("Record"))
            .on_press(Message::StartRecord)
    };

    let level = (audio_level / 10.0) as usize;
    let level_bar = "█".repeat(level)
        + &"░".repeat((10 - level).max(0));

    let bottom_bar = Container::new(
        Row::new()
            .push(rec_btn)
            .push(Text::new(format!(" Level: {}", level_bar)).size(12))
            .spacing(16)
            .padding(8),
    );

    Container::new(bottom_bar)
        .width(Length::Fill)
        .height(50)
        .into()
}
