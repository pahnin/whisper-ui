pub mod app;
pub mod document;
pub mod workspace;
pub mod inference;
pub mod audio;
pub mod ui;

fn subscription(state: &crate::app::AppState) -> iced::Subscription<crate::app::Message> {
    if state.is_recording {
        iced::Subscription::run(tick_stream)
    } else {
        iced::Subscription::none()
    }
}

fn tick_stream() -> impl futures_lite::stream::Stream<Item = crate::app::Message> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let _ = tx.send(crate::app::Message::PollTrigger);
        }
    });
    futures_lite::stream::unfold(rx, |rx| async move {
        loop {
            match rx.try_recv() {
                Ok(msg) => return Some((msg, rx)),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    futures_lite::future::yield_now().await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return None,
            }
        }
    })
}

fn main() {
    let language_options = {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            crate::inference::backend::model_manager::ModelManager::language_options()
        })
    };

    let boot = move || {
        let base_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
            .unwrap_or_else(|_| std::path::PathBuf::from("whisper-app-data"));

        let workspace = crate::workspace::Workspace::load(&base_dir);

        let mut app_state = crate::app::AppState::default();
        app_state.workspace = workspace;

        let (model_idx, language, last_active) = crate::app::load_app_state();
        app_state.selected_model_idx = model_idx;
        app_state.language = language;
        app_state.last_active_doc = last_active;

        app_state.load_models();
        app_state.language_options = language_options.clone();

        if app_state.selected_model_idx < app_state.models.len() {
            if app_state.models[app_state.selected_model_idx].downloaded {
                app_state.model_status = app::ModelStatus::Ready;
                let _ = app_state.init_backend();
            }
        }

        let has_downloaded = app_state.models.iter().any(|m| m.downloaded);
        app_state.show_landing = !has_downloaded;

        app_state
    };

    let _ = iced::application(boot, crate::app::update, crate::app::view)
        .subscription(subscription)
        .run();
}
