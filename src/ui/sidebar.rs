use iced::widget::{button, column, Column, Container, Row, Scrollable, Text, TextInput};
use iced::widget::button::Style as ButtonStyle;
use iced::widget::container;
use iced::widget::container::Style as ContainerStyle;
use iced::{Element, Length};

use crate::app::Message;
use crate::workspace::Workspace;

fn sidebar_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_: &iced::Theme| {
        container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.12, g: 0.12, b: 0.14, a: 1.0,
            })),
            border: iced::Border {
                radius: 4.0.into(),
                width: 0.5,
                color: iced::Color {
                    r: 0.18, g: 0.18, b: 0.22, a: 0.5,
                },
            },
            shadow: iced::Shadow::default(),
            snap: false,
            text_color: Some(iced::Color {
                r: 0.93, g: 0.93, b: 0.95, a: 1.0,
            }),
        }
    }
}

fn settings_text_style() -> impl Fn(&iced::Theme) -> container::Style {
    |_| ContainerStyle {
        text_color: Some(iced::Color {
            r: 0.55, g: 0.55, b: 0.60, a: 1.0,
        }),
        ..ContainerStyle::default()
    }
}

fn action_btn_style(is_hovered: bool) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> ButtonStyle {
    move |_theme, _status| ButtonStyle {
        text_color: if is_hovered {
            iced::Color { r: 0.80, g: 0.80, b: 0.85, a: 1.0 }
        } else {
            iced::Color { r: 0.45, g: 0.45, b: 0.50, a: 1.0 }
        },
        ..ButtonStyle::default()
    }
}

pub fn view<'a>(
    workspace: &'a Workspace,
    active_id: Option<uuid::Uuid>,
    rename_doc: Option<uuid::Uuid>,
    rename_input: &'a str,
    hovered_doc: Option<uuid::Uuid>,
) -> Element<'a, Message> {
    let mut doc_items = Column::new().spacing(2);

    for (_, doc) in &workspace.documents {
        let is_active = active_id == Some(doc.id);
        let is_hovered = hovered_doc == Some(doc.id);
        let is_renaming = rename_doc == Some(doc.id);

        let row: Element<'_, Message> = if is_renaming {
            let rename_text = TextInput::new("Rename...", rename_input)
                .on_input(Message::RenameDocumentConfirm)
                .on_submit(Message::RenameDocumentConfirm(rename_input.to_string()))
                .size(12);

            let styled_input = Container::new(rename_text)
                .width(Length::Fill);

            let cancel_btn = button(Text::new("\u{2715}"))
                .on_press(Message::RenameDocumentConfirm(rename_input.to_string()))
                .style(action_btn_style(false));

            let actions = Row::new()
                .push(cancel_btn)
                .width(28)
                .align_y(iced::Alignment::Center);

            Row::new()
                .push(styled_input)
                .push(actions)
                .spacing(2)
                .height(28)
                .align_y(iced::Alignment::Center)
                .into()
        } else {
            let accent_bar = Container::new(Container::new(Text::new(" ")).width(2))
                .width(2)
                .height(Length::Fill)
                .style(move |_theme: &iced::Theme| {
                    if is_active {
                        container::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                r: 0.55, g: 0.45, b: 0.75, a: 1.0,
                            })),
                            ..container::Style::default()
                        }
                    } else {
                        container::Style::default()
                    }
                });

            let title = Text::new(&doc.title).size(12);
            let styled_title = Container::new(title)
                .width(Length::Fill)
                .style(move |_theme: &iced::Theme| {
                    container::Style {
                        text_color: if is_active {
                            Some(iced::Color {
                                r: 0.93, g: 0.93, b: 0.95, a: 1.0,
                            })
                        } else {
                            Some(iced::Color {
                                r: 0.55, g: 0.55, b: 0.60, a: 1.0,
                            })
                        },
                        ..container::Style::default()
                    }
                });

            let rename_icon = button(Text::new("\u{270E}"))
                .on_press(Message::RenameDocument(doc.id))
                .style(action_btn_style(false));

            let delete_icon = button(Text::new("\u{2715}"))
                .on_press(Message::DeleteDocument(doc.id))
                .style(action_btn_style(false));

            let actions = Row::new()
                .push(rename_icon)
                .push(delete_icon)
                .width(40)
                .align_y(iced::Alignment::Center);

            Row::new()
                .push(accent_bar)
                .push(styled_title)
                .push(actions)
                .spacing(4)
                .height(28)
                .align_y(iced::Alignment::Center)
                .into()
        };

        let bg_color = if is_active {
            Some(iced::Color { r: 0.18, g: 0.18, b: 0.22, a: 1.0 })
        } else if is_hovered {
            Some(iced::Color { r: 0.16, g: 0.16, b: 0.19, a: 1.0 })
        } else {
            None
        };

        let styled_row = Container::new(row)
            .width(Length::Fill)
            .height(28)
            .style(move |_theme: &iced::Theme| {
                container::Style {
                    background: bg_color.map(|c| iced::Background::Color(c)),
                    border: if is_hovered && !is_active {
                        iced::Border {
                            radius: 2.0.into(),
                            width: 0.5,
                            color: iced::Color {
                                r: 0.25, g: 0.25, b: 0.30, a: 1.0,
                            },
                        }
                    } else {
                        iced::Border {
                            radius: 2.0.into(),
                            width: 0.0,
                            color: iced::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
                        }
                    },
                    ..container::Style::default()
                }
            });

        doc_items = doc_items.push(styled_row);
    }

    let new_doc_btn = button(
        Container::new(Text::new("New Document").size(12))
            .padding(6),
    )
    .on_press(Message::NewDocument);

    let settings_btn = button(
        Container::new(Text::new("Settings").size(12))
            .padding(4),
    )
    .on_press(Message::ShowSettings);

    let settings_container = Container::new(settings_btn)
        .style(settings_text_style());

    let heading = Text::new("Documents").size(13);

    let sidebar_content = column![
        heading,
        new_doc_btn,
        Scrollable::new(doc_items).height(Length::Fill),
        settings_container,
    ]
    .spacing(12)
    .padding(iced::Padding::new(12.0).top(16.0).bottom(16.0));

    Container::new(sidebar_content)
        .width(250)
        .height(Length::Fill)
        .style(sidebar_style())
        .into()
}
