extern crate gtk4;

use std::sync::{Arc, Mutex};

use gtk4::{Application, CssProvider, glib, IconTheme};
use gtk4::gdk::Display;
use gtk4::gio::resources_register_include;
use gtk4::prelude::{ApplicationExt, ApplicationExtManual};
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use database::{connect, model::library_entry::Model as LibraryEntry};
use player::{Player, Progress};

use crate::components::{Component, WindowComponent};
use crate::splash::Splash;
use crate::state::{Action, Dispatcher, Event, EventHandler, State};

mod state;
mod components;
mod util;
mod splash;
// mod gui;

const APP_ID: &str = "org.tinyghettobox.gui";

#[tokio::main]
async fn main() -> glib::ExitCode {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                Targets::new().with_default(LevelFilter::DEBUG)
                    .with_target("ureq", LevelFilter::INFO)
                // .with_target("user_interface::state", LevelFilter::INFO)
            )
        )
        .init();

    // let splash = Splash::new();

    info!("Starting user interface");

    resources_register_include!("composite_templates.gresource").expect("Failed to register resources.");

    let connection = connect().await.expect("Could not connect to database");
    let state = Arc::new(Mutex::new(State::new(connection.clone()).await));
    let dispatcher = Arc::new(Mutex::new(Dispatcher::new()));
    let player = Player::new(connection.clone(), state.lock().unwrap().volume).await;

    {
        let player = player.clone();
        let dispatcher1 = dispatcher.clone();
        let dispatcher2 = dispatcher.clone();
        tokio::spawn(async move {
            let mut player = player.lock().await;
            let handle_progress_change = move |progress: Progress| {
                dispatcher1
                    .clone()
                    .lock()
                    .unwrap()
                    .dispatch_action(Action::SetProgress(progress.as_f64()));
            };
            let handle_track_change = move |library_entry: Option<LibraryEntry>| {
                dispatcher2
                    .lock()
                    .unwrap()
                    .dispatch_action(Action::SetPlayingTrack(library_entry));
            };

            player.connect_progress_change(handle_progress_change);
            player.connect_track_change(handle_track_change);
        });
    }


    let handle = tokio::runtime::Handle::current();
    let thread = std::thread::spawn(move || {
        handle.block_on(async {
            let app = Application::builder().application_id(APP_ID).build();
            app.connect_startup(|_| {
                info!("Startup");
                let theme = IconTheme::for_display(&Display::default().unwrap());
                theme.add_resource_path("/org/tinyghettobox/gui/icons/scalable/actions/");
                theme.add_search_path("/org/tinyghettobox/gui/icons/scalable/actions/");

                let provider = CssProvider::new();
                provider.load_from_resource("/org/tinyghettobox/gui/styles.css");

                gtk4::style_context_add_provider_for_display(
                    &Display::default().expect("Could not connect to a display."),
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
                info!("Startup done");
            });
            app.connect_activate(move |app: &Application| {
                let dispatcher = dispatcher.clone();
                let state = state.clone();
                let player = player.clone();

                info!("Render app");
                let window = WindowComponent::new(state.clone(), dispatcher.clone(), None);
                window.present(app);

                let window = Arc::new(Mutex::new(Box::new(window) as Box<dyn EventHandler>));

                let dispatcher_clone = dispatcher.clone();
                dispatcher.lock().unwrap().handle(
                    move |action| {
                        let dispatcher = dispatcher_clone.clone();
                        let state = state.clone();
                        let player = player.clone();
                        Action::process(action, state.clone(), dispatcher.clone(), player.clone())
                    },
                    move |event| {
                        Event::broadcast(event, window.clone())
                    },
                );

                info!("Rendered");
            });

            info!("Run");
            // Run the application
            app.run()
        })
    });

    thread.join().unwrap()
}
