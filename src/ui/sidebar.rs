use iced::widget::{button, column, Column, Container, Scrollable, Text};
use iced::{Element, Length};

use crate::app::Message;
use crate::workspace::Workspace;

pub fn view<'a>(
    workspace: &'a Workspace,
    active_id: Option<uuid::Uuid>,
) -> Element<'a, Message> {
    let mut doc_items = Column::new()
        .spacing(4)
        .push(
            button(Text::new("+ New Document"))
                .on_press(Message::NewDocument),
        );

    for (_, doc) in &workspace.documents {
        let is_active = active_id.map(|id| id == doc.id).unwrap_or(false);
        let label = if is_active {
            format!("▸ {}", doc.title)
        } else {
            format!("  {}", doc.title)
        };

        doc_items = doc_items.push(
            button(Text::new(label))
                .on_press(Message::SelectDocument(doc.id)),
        );
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
        .into()
}
