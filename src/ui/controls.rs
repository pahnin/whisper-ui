use iced::widget::{button, Container, Row, Text};
use iced::widget::container;
use iced::Length;

use crate::app::Message;

fn controls_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.12,
                g: 0.12,
                b: 0.14,
                a: 1.0,
            })),
            border: iced::Border {
                radius: 0.0.into(),
                width: 1.0,
                color: iced::Color {
                    r: 0.2,
                    g: 0.2,
                    b: 0.25,
                    a: 1.0,
                },
            },
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: Some(iced::Color::WHITE),
        }
    }
}

pub fn view<'a>(
    is_recording: bool,
    is_paused: bool,
    audio_level: f32,
    backend_ready: bool,
) -> iced::Element<'a, Message> {
    let rec_btn = if !backend_ready {
        button(Text::new("Load Model First"))
    } else if is_recording && !is_paused {
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

    let status_text = if !backend_ready {
        Text::new(" ⚠ No model loaded - configure in Settings").size(12)
    } else if is_recording && !is_paused {
        Text::new("● REC").size(12)
    } else {
        Text::new(format!(" Level: {}", level_bar)).size(12)
    };

    let bottom_bar = Container::new(
        Row::new()
            .push(rec_btn)
            .push(status_text)
            .spacing(16)
            .padding(8),
    );

    Container::new(bottom_bar)
        .width(Length::Fill)
        .height(50)
        .style(controls_style())
        .into()
}
