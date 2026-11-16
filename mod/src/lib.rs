//! Horizon Quests: an FTB Quests-style quest book for Horizon, written
//! against Horizon's mod API (ADR-018).
//!
//! The `hq_*` modules are the pure logic: no game types, so `cargo test` runs
//! them. The two glue modules carry it to the game: `hq_server_glue` runs the
//! quests on the server, `hq_client_glue` draws the book on the client. The one
//! `impl Hooks` below only routes each moment of the game to them.

pub mod hq_book;
pub mod hq_built_in;
pub mod hq_canvas;
pub mod hq_commands;
pub mod hq_config;
pub mod hq_edit;
pub mod hq_inline;
pub mod hq_logic;
pub mod hq_meta;
pub mod hq_model;
pub mod hq_net;
pub mod hq_progress;
pub mod hq_session;
pub mod hq_store;
pub mod hq_teams;
pub mod hq_text;
pub mod hq_validate;

mod hq_client_glue;
mod hq_server_glue;

#[cfg(test)]
mod tests;

use hq_client_glue as client;
use hq_server_glue as server;
use mod_api::{
    AddServerReloadListenersEvent, ClientHost, ClientTickPostEvent, ConfigLoadingEvent, ConfigReloadingEvent,
    ConfigUnloadingEvent, Flow, FurnaceExtractEvent, Hooks, Host, HostSide, ItemCraftedEvent, MobKilledEvent,
    PlayerAdvancementDoneEvent, PlayerInventoryChangedEvent, PlayerLevelChangeEvent, PlayerLoggedInEvent,
    PlayerLoggedOutEvent, RegisterCommandsEvent, RegisterConfigsEvent, Registrar, RegistrationContext,
    RenderGuiPostEvent, RunModCommandEvent, ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenMouseButtonReleasedPreEvent, ScreenMouseDraggedPreEvent,
    ScreenMouseScrolledPreEvent, ScreenRenderPostEvent, ServerHost, ServerStartedEvent, ServerStoppedEvent,
    ServerTickEvent,
};

#[derive(Default)]
struct Mod;

/// A screen hook's answer: whether the book took the input.
fn flow(taken: bool) -> Flow {
    if taken { Flow::Cancel } else { Flow::Continue }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// At startup: the open key and the book's channel.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        client::register(r);
        hq_net_glue::register(r);
    }

    // ---- Config files ----

    fn register_configs(&self, _host: HostSide, event: &mut RegisterConfigsEvent<'_>) {
        event.configs.push(server::config_file());
        event.configs.push(client::config_file());
    }

    fn config_loading(&self, _host: HostSide, event: &mut ConfigLoadingEvent<'_>) {
        server::config_read(event.config, false);
        client::config_read(event.config);
    }

    fn config_reloading(&self, _host: HostSide, event: &mut ConfigReloadingEvent<'_>) {
        server::config_read(event.config, true);
        client::config_read(event.config);
    }

    fn config_unloading(&self, _host: &mut dyn ServerHost, event: &mut ConfigUnloadingEvent<'_>) {
        server::config_dropped(event.config);
    }

    // ---- Server: the quest engine ----

    fn add_server_reload_listeners(&self, _host: &dyn Host, event: &mut AddServerReloadListenersEvent<'_>) {
        server::reload_listeners(event.listeners);
    }

    fn server_started(&self, host: &mut dyn ServerHost, _event: &mut ServerStartedEvent) {
        server::world_opened(host);
    }

    fn server_stopped(&self, _host: &mut dyn ServerHost, _event: &mut ServerStoppedEvent) {
        server::world_closed();
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        server::server_tick(host, event.phase);
    }

    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        server::logged_in(host, event.player, event.name);
    }

    fn player_logged_out(&self, _host: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        server::logged_out(event.player);
    }

    fn player_inventory_changed(&self, _host: &mut dyn ServerHost, event: &mut PlayerInventoryChangedEvent<'_>) {
        server::poke(event.player);
    }

    fn mob_killed(&self, _host: &mut dyn ServerHost, event: &mut MobKilledEvent<'_>) {
        server::mob_killed(event.player, event.kind);
    }

    fn item_crafted(&self, host: &mut dyn ServerHost, event: &mut ItemCraftedEvent<'_>) {
        let item = event.stack.item().id(host);
        server::crafted(event.player, &item, event.stack.count());
    }

    fn furnace_extract(&self, _host: &mut dyn ServerHost, event: &mut FurnaceExtractEvent<'_>) {
        server::crafted(event.player, event.item, event.count);
    }

    fn player_level_change(&self, _host: &mut dyn ServerHost, event: &mut PlayerLevelChangeEvent<'_>) {
        server::level_changed(event.player, event.to);
    }

    fn player_advancement_done(&self, _host: &mut dyn ServerHost, event: &mut PlayerAdvancementDoneEvent<'_>) {
        server::advancement_done(event.player, event.advancement);
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        server::commands(event.commands);
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        flow(server::perform(host, event.source, event.command))
    }

    // ---- Client: the quest book ----

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        client::tick(host);
    }

    fn render_gui_post(&self, host: &dyn ClientHost, event: &mut RenderGuiPostEvent<'_>) {
        client::hud(host, event.draw, event.corners);
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        client::render(host, event.draw, event.screen);
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        flow(client::pressed(host, event.screen, event.button))
    }

    fn screen_mouse_button_released_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseButtonReleasedPreEvent<'_>) -> Flow {
        flow(client::released(host, event.screen, event.button))
    }

    fn screen_mouse_dragged_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseDraggedPreEvent<'_>) -> Flow {
        flow(client::dragged(host, event.screen, event.delta))
    }

    fn screen_mouse_scrolled_pre(&self, host: &dyn ClientHost, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        flow(client::scrolled(host, event.screen, event.scroll))
    }

    fn screen_key_pressed_pre(&self, host: &dyn ClientHost, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        flow(client::screen_key(host, event.screen, event.key))
    }

    fn screen_char_typed_pre(&self, host: &dyn ClientHost, event: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
        flow(client::typed(host, event.c))
    }
}

/// The book's channel, `horizon_quests:book`, both ways (hq_net frames): the
/// server's half hands what a client sends to `hq_server_glue`, the client's
/// half what the server sends to `hq_client_glue`.
mod hq_net_glue {
    use super::hq_meta as meta;
    use super::hq_net as net;
    use mod_api::{IPayloadContext, IPayloadHandler, PayloadBytes, PayloadHandleError, PayloadId, Registrar};
    use std::sync::Arc;

    pub(super) fn channel() -> PayloadId {
        PayloadId(net::CHANNEL.to_owned())
    }

    pub(super) fn register(r: &mut dyn Registrar) {
        let version = net::PROTOCOL.to_string();
        r.payload_handlers().registrar(&version).play_bidirectional(channel(), Arc::new(FromClient), Some(Arc::new(FromServer)));
    }

    fn failed(e: impl std::fmt::Debug) -> PayloadHandleError {
        PayloadHandleError::Failed(format!("{}: {e:?}", meta::NAME))
    }

    struct FromClient;
    impl IPayloadHandler for FromClient {
        fn handle(&self, payload: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), PayloadHandleError> {
            let player = context.player().map_err(failed)?.uuid();
            let host = context.server_host().map_err(failed)?;
            super::server::from_client(host, player, &payload.0);
            Ok(())
        }
    }

    struct FromServer;
    impl IPayloadHandler for FromServer {
        fn handle(&self, payload: PayloadBytes, context: &mut dyn IPayloadContext) -> Result<(), PayloadHandleError> {
            let host = context.client_host().map_err(failed)?;
            super::client::from_server(host, &payload.0);
            Ok(())
        }
    }
}
