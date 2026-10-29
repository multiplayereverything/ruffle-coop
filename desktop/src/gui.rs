mod context_menu;
mod controller;
pub mod dialogs;
mod locale;
mod menu_bar;
mod movie;
mod picker;
mod theme;
mod widgets;

pub use controller::GuiController;
pub use dialogs::DialogDescriptor;
pub use locale::LocalizableText;
pub use locale::available_languages;
pub use locale::optional_text;
pub use locale::text;
pub use locale::text_with_args;
pub use movie::MovieView;
pub use picker::FilePicker;
use ruffle_frontend_utils::content::ContentDescriptor;
pub use theme::ThemePreference;

use crate::custom_event::RuffleEvent;
use crate::gui::context_menu::ContextMenu;
use crate::player::LaunchOptions;
use crate::preferences::GlobalPreferences;
use dialogs::Dialogs;
use egui::*;
use menu_bar::MenuBar;
use rfd::AsyncFileDialog;
use ruffle_core::debug_ui::Message as DebugMessage;
use ruffle_core::{Player, PlayerEvent};
use std::sync::{MutexGuard, Weak};
use std::{fs, mem};
use winit::event_loop::EventLoopProxy;

/// Size of the top menu bar in pixels.
/// This is the offset at which the movie will be shown,
/// and added to the window size if trying to match a movie.
pub const MENU_HEIGHT: u32 = 24;

/// The main controller for the Ruffle GUI.
pub struct RuffleGui {
    event_loop: EventLoopProxy<RuffleEvent>,
    context_menu: Option<ContextMenu>,
    dialogs: Dialogs,
    menu_bar: MenuBar,

    was_suspended_before_debug: bool,
    preferences: GlobalPreferences,
}

impl RuffleGui {
    fn new(
        window: Weak<winit::window::Window>,
        event_loop: EventLoopProxy<RuffleEvent>,
        default_content: Option<ContentDescriptor>,
        default_launch_options: LaunchOptions,
        preferences: GlobalPreferences,
    ) -> Self {
        Self {
            was_suspended_before_debug: false,

            context_menu: None,
            dialogs: Dialogs::new(
                preferences.clone(),
                default_launch_options.clone(),
                default_content,
                window,
                event_loop.clone(),
            ),
            menu_bar: MenuBar::new(
                event_loop.clone(),
                default_launch_options,
                preferences.clone(),
            ),

            event_loop,
            preferences,
        }
    }

    /// Renders all of the main Ruffle UI, including the main menu and context menus.
    fn update(
        &mut self,
        egui_ctx: &egui::Context,
        egui_ui: &mut egui::Ui,
        show_menu: bool,
        mut player: Option<&mut Player>,
        menu_height_offset: f64,
    ) {
        let locale = self.preferences.language();

        self.menu_bar
            .consume_shortcuts(egui_ctx, &mut self.dialogs, player.as_deref_mut());
        if show_menu {
            self.menu_bar
                .show(&locale, egui_ui, &mut self.dialogs, player.as_deref_mut());
        }

        self.dialogs.show(&locale, egui_ctx, player.as_deref_mut());

        if let Some(lobby) = crate::coop::waiting_lobby() {
            show_lobby(egui_ui, &lobby);
        }

        if let Some(player) = player.as_deref_mut() {
            let labels = crate::coop::session().map(|session| session.labels()).unwrap_or_default();
            show_labels(egui_ctx, player, &labels, menu_height_offset);
        }

        if let Some(player) = player {
            let was_suspended = player.debug_ui().should_suspend_player();
            player.show_debug_ui(egui_ctx, menu_height_offset);
            if was_suspended != player.debug_ui().should_suspend_player() {
                if player.debug_ui().should_suspend_player() {
                    self.was_suspended_before_debug = !player.is_playing();
                    player.set_is_playing(false);
                } else {
                    player.set_is_playing(!self.was_suspended_before_debug);
                }
            }
            for item in player.debug_ui().items_to_save() {
                let dialog = AsyncFileDialog::new().set_file_name(&item.suggested_name);
                let picker = self.dialogs.file_picker();
                let result = picker.show_dialog(dialog, |d| d.save_file());
                if let Some(result) = result {
                    tokio::spawn(async move {
                        let Some(handle) = result.await else {
                            return;
                        };
                        let path = handle.path();
                        if let Err(e) = fs::write(path, item.data) {
                            tracing::error!(
                                "Couldn't save {} to {path:?}: {e}",
                                item.suggested_name,
                            );
                        }
                    });
                }
            }

            if let Some(context_menu) = &mut self.context_menu
                && !context_menu.show(&locale, egui_ctx, &self.event_loop, player.is_fullscreen())
            {
                self.close_context_menu(player);
            }
        };
    }

    pub fn show_context_menu(
        &mut self,
        menu: Vec<ruffle_core::ContextMenuItem>,
        close_event: PlayerEvent,
    ) {
        if !menu.is_empty() {
            self.context_menu = Some(ContextMenu::new(menu, close_event));
        }
    }

    pub fn close_context_menu(&mut self, player: &mut Player) {
        if let Some(context_menu) = mem::take(&mut self.context_menu) {
            player.handle_event(context_menu.close_event());
        }
    }

    pub fn is_context_menu_visible(&self) -> bool {
        self.context_menu.is_some()
    }

    /// Notifies the GUI that the player has been destroyed.
    fn on_player_destroyed(&mut self) {
        self.dialogs.close_dialogs_with_notifiers();
    }

    /// Notifies the GUI that a new player was created.
    fn on_player_created(
        &mut self,
        opt: LaunchOptions,
        content_descriptor: ContentDescriptor,
        mut player: MutexGuard<Player>,
    ) {
        self.menu_bar.currently_opened = Some((content_descriptor.clone(), opt.clone()));

        // Update dialog state to reflect the newly-opened movie's options.
        self.dialogs
            .recreate_open_dialog(opt, Some(content_descriptor), self.event_loop.clone());

        player.set_volume(self.dialogs.volume_controls.get_volume());
    }
}

/// Ruffle Together: the game's name tags (`coop.label`), above their clips as this PC's camera
/// shows them. They are drawn over the game, never in it, so they can't change the game.
fn show_labels(
    egui_ctx: &egui::Context,
    player: &mut Player,
    labels: &[crate::coop::api::Label],
    menu_height_offset: f64,
) {
    if labels.is_empty() {
        return;
    }
    let painter = egui_ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("coop-labels")));
    let points = egui_ctx.pixels_per_point() as f64;
    for label in labels {
        let Some((x, y)) = player.coop_label_position(&label.path) else {
            continue;
        };
        let at = pos2((x / points) as f32, ((y + menu_height_offset) / points) as f32 - 4.0);
        let font = FontId::proportional(14.0);
        painter.text(at + vec2(1.0, 1.0), Align2::CENTER_BOTTOM, &label.text, font.clone(), Color32::BLACK);
        painter.text(at, Align2::CENTER_BOTTOM, &label.text, font, Color32::WHITE);
    }
}

/// Ruffle Together: the window's waiting screen until everyone has joined the session.
fn show_lobby(ui: &mut Ui, lobby: &crate::coop::Lobby) {
    CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.2);
            ui.heading("Ruffle Together");
            ui.add_space(16.0);
            if let Some(error) = &lobby.error {
                ui.label(RichText::new("Can't start the game").size(20.0).strong());
                ui.label(error);
                ui.add_space(8.0);
                ui.label("Close this window and try again.");
                return;
            }
            if !lobby.connected {
                ui.label(if lobby.hosting { "Opening the room..." } else { "Connecting..." });
                return;
            }
            if let Some(code) = &lobby.room {
                ui.label(if lobby.hosting { "Your room code" } else { "Room" });
                ui.label(RichText::new(code).size(48.0).monospace().strong());
            } else if let Some(port) = lobby.port {
                ui.label(format!("Hosting on port {port}: friends join with your IP address and this port."));
            }
            ui.add_space(16.0);
            if lobby.hosting {
                let waiting = format!("Waiting for players: {} of {}", lobby.players.len(), lobby.needed);
                ui.label(RichText::new(waiting).size(18.0));
                for (i, name) in lobby.players.iter().enumerate() {
                    ui.label(format!("Player {}: {name}", i + 1));
                }
                ui.add_space(8.0);
                if lobby.room.is_some() {
                    ui.label("Send the room code to your friends. The game starts when everyone has joined.");
                } else {
                    ui.label("The game starts when everyone has joined.");
                }
            } else {
                ui.label(RichText::new("Waiting for the host to start the game...").size(18.0));
            }
        });
    });
}
