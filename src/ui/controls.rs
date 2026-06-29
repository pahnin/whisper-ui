use iced::widget::{button, Container, Row, Space, Text};
use iced::widget::container;
use iced::Length;

use crate::app::Message;

fn button_style(
    is_recording: bool,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_: &iced::Theme, _status| {
        let bg = if is_recording {
            iced::Color {
                r: 0.85,
                g: 0.30,
                b: 0.35,
                a: 1.0,
            }
        } else {
            iced::Color {
                r: 0.55,
                g: 0.45,
                b: 0.75,
                a: 1.0,
            }
        };

        iced::widget::button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                radius: 20.0.into(),
                width: 1.0,
                color: iced::Color {
                    r: 0.30,
                    g: 0.30,
                    b: 0.36,
                    a: 1.0,
                },
            },
            text_color: iced::Color::WHITE,
            shadow: iced::Shadow::default(),
            snap: false,
        }
    }
}

fn controls_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.10,
                g: 0.10,
                b: 0.12,
                a: 1.0,
            })),
            border: iced::Border {
                radius: 3.0.into(),
                width: 0.0,
                color: iced::Color {
                    r: 0.18,
                    g: 0.18,
                    b: 0.22,
                    a: 0.3,
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
    _audio_level: f32,
    model_loaded: bool,
    accelerator: Option<&'a str>,
) -> iced::Element<'a, Message> {
    let rec_text = if !model_loaded {
        "Record"
    } else if is_recording && !is_paused {
        "Stop"
    } else if is_paused {
        "Resume"
    } else {
        "Record"
    };

    let is_recording_active = is_recording && !is_paused;

    let rec_btn = button(Text::new(rec_text).size(12))
        .on_press(if !model_loaded || is_recording_active {
            Message::StopRecord
        } else if is_paused {
            Message::ResumeRecord
        } else {
            Message::StartRecord
        })
        .style(button_style(is_recording_active))
        .padding([3.0, 18.0]);

    let acc_text = if let Some(acc) = accelerator {
        format!("[{}]", acc)
    } else {
        String::new()
    };

    let indicator_color = if is_recording_active {
        iced::Color {
            r: 0.85,
            g: 0.30,
            b: 0.35,
            a: 1.0,
        }
    } else if !model_loaded {
        iced::Color {
            r: 0.42,
            g: 0.42,
            b: 0.48,
            a: 1.0,
        }
    } else {
        iced::Color {
            r: 0.55,
            g: 0.45,
            b: 0.75,
            a: 1.0,
        }
    };

    let dot = Text::new("\u{25CF}").size(12);
    let dot_container = Container::new(dot)
        .width(16)
        .height(16)
        .align_x(iced::Alignment::Center)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(indicator_color)),
            border: iced::Border {
                radius: 8.0.into(),
                width: 0.0,
                color: iced::Color::WHITE,
            },
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: None,
        });

    let status_text = if !model_loaded {
        Text::new("No model loaded")
            .size(11)
            .color(iced::Color {
                r: 0.42,
                g: 0.42,
                b: 0.48,
                a: 1.0,
            })
    } else if is_recording_active {
        Text::new("REC")
            .size(11)
            .color(indicator_color)
    } else {
        Text::new("Ready")
            .size(11)
            .color(iced::Color {
                r: 0.42,
                g: 0.42,
                b: 0.48,
                a: 1.0,
            })
    };

    let acc_label = if !acc_text.is_empty() {
        Text::new(acc_text)
            .size(10)
            .color(iced::Color {
                r: 0.30,
                g: 0.30,
                b: 0.35,
                a: 1.0,
            })
    } else {
        Text::new("")
            .size(10)
            .color(iced::Color::TRANSPARENT)
    };

    let status_row = Row::new()
        .push(dot_container)
        .push(status_text)
        .push(Space::new().width(Length::Fixed(4.0)))
        .push(acc_label)
        .spacing(6)
        .align_y(iced::Alignment::Center);

    let bottom_bar = Row::new()
        .push(rec_btn)
        .push(status_row)
        .push(Space::new().width(Length::Fill))
        .spacing(16)
        .align_y(iced::Alignment::Center)
        .padding(12);

    Container::new(bottom_bar)
        .width(Length::Fill)
        .height(40)
        .style(controls_style())
        .into()
}
