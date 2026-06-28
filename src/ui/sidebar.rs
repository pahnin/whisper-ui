use iced::widget::{button, column, Column, Container, Scrollable, Text, TextInput};
use iced::widget::container;
use iced::{Element, Length};

use crate::app::Message;
use crate::workspace::Workspace;

fn sidebar_style() -> impl Fn(&iced::Theme) -> container::Style {
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
    workspace: &'a Workspace,
    active_id: Option<uuid::Uuid>,
    rename_doc: Option<uuid::Uuid>,
    rename_input: &'a str,
) -> Element<'a, Message> {
    let mut doc_items = Column::new()
        .spacing(4)
        .push(
            button(Text::new("+ New Document"))
                .on_press(Message::NewDocument),
        );

    for (_, doc) in &workspace.documents {
        let is_active = active_id.map(|id| id == doc.id).unwrap_or(false);
        let is_renaming = rename_doc.map(|id| id == doc.id).unwrap_or(false);

        if is_renaming {
            let item: Element<Message> = Column::new()
                .push(
                    TextInput::new("Rename...", &rename_input)
                        .on_input(Message::RenameDocumentConfirm)
                        .on_submit(Message::RenameDocumentConfirm(rename_input.to_string()))
                        .width(180.0)
                        .size(12),
                )
                .into();
            doc_items = doc_items.push(item);
        } else {
            let label = if is_active {
                format!("▸ {}", doc.title)
            } else {
                format!("  {}", doc.title)
            };

            let button_content = button(Text::new(label))
                .on_press(Message::SelectDocument(doc.id));

            let rename_btn = button(Text::new(" ✎"))
                .on_press(Message::RenameDocument(doc.id));

            let row: Element<Message> = Column::new()
                .push(
                    Column::new()
                        .push(button_content)
                        .push(rename_btn),
                )
                .into();

            doc_items = doc_items.push(row);
        }
    }

    let sidebar_content = column![
        Text::new("Documents").size(16),
        Scrollable::new(doc_items).height(Length::Fill),
        button(Text::new("Settings"))
            .on_press(Message::ShowSettings),
    ]
    .spacing(8)
    .padding(12);

    Container::new(sidebar_content)
        .width(250)
        .style(sidebar_style())
        .into()
}
