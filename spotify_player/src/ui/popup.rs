use crate::{command::Command, utils::filtered_items_from_query};

use super::{
    config, utils, utils::construct_and_render_block, Borders, Cell, Constraint, Frame, Layout,
    Paragraph, PlaylistCreateCurrentField, PlaylistPopupAction, PopupState, Rect, Row, SharedState,
    Table, UIStateGuard,
};

const SHORTCUT_TABLE_N_COLUMNS: usize = 3;
const SHORTCUT_TABLE_CONSTRAINS: [Constraint; SHORTCUT_TABLE_N_COLUMNS] =
    [Constraint::Ratio(1, 3); 3];

/// Render a popup (if any) to handle a command or show additional information
/// depending on the current popup state.
///
/// The function returns a rectangle area to render the main layout and
/// a boolean value determining whether the focus should be placed in the main layout.
pub fn render_popup(
    frame: &mut Frame,
    state: &SharedState,
    ui: &mut UIStateGuard,
    rect: Rect,
) -> (Rect, bool) {
    match ui.popup {
        None => {
            // Popup is gone (however it was closed - Esc, choosing an action,
            // clicking elsewhere) - drop any stale click position so the next
            // popup doesn't accidentally reuse an old anchor.
            ui.mouse_popup_anchor = None;
            (rect, true)
        }
        Some(ref popup) => match popup {
            PopupState::PlaylistCreate {
                name,
                desc,
                current_field,
            } => {
                let chunks =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(rect);

                let popup_chunks =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(chunks[1]);

                let name_input = construct_and_render_block(
                    "Enter Name for New Playlist:",
                    &ui.theme,
                    Borders::ALL,
                    frame,
                    popup_chunks[0],
                );

                let desc_input = construct_and_render_block(
                    "Enter Description for New Playlist:",
                    &ui.theme,
                    Borders::ALL,
                    frame,
                    popup_chunks[1],
                );

                frame.render_widget(
                    name.widget(PlaylistCreateCurrentField::Name == *current_field),
                    name_input,
                );
                frame.render_widget(
                    desc.widget(PlaylistCreateCurrentField::Desc == *current_field),
                    desc_input,
                );
                (chunks[0], true)
            }
            PopupState::Search { query } => {
                let chunks =
                    Layout::vertical([Constraint::Fill(0), Constraint::Length(3)]).split(rect);

                let rect =
                    construct_and_render_block("Search", &ui.theme, Borders::ALL, frame, chunks[1]);

                frame.render_widget(Paragraph::new(format!("/{query}")), rect);
                (chunks[0], true)
            }
            PopupState::ActionList(item, _) => {
                let title = format!("Actions on {}", item.name());
                let items = item
                    .actions_desc()
                    .into_iter()
                    .enumerate()
                    .map(|(id, d)| (format!("[{id}] {d}"), false))
                    .collect::<Vec<_>>();
                let height = item.n_actions() as u16 + 2; // 2 for top/bot paddings

                let rect = match ui.mouse_popup_anchor {
                    // Opened via right-click: render right where the user clicked.
                    Some((x, y)) => {
                        render_positioned_list_popup(frame, rect, x, y, &title, items, height, ui)
                    }
                    // Opened via keyboard (the "a" keybind): keep the original
                    // bottom-docked behavior.
                    None => render_list_popup(frame, rect, &title, items, height, ui),
                };
                (rect, false)
            }
            PopupState::DeviceList { .. } => {
                let player = state.player.read();

                let current_device_id = match player.playback {
                    Some(ref playback) => playback.device.id.as_deref().unwrap_or_default(),
                    None => "",
                };
                let items = player
                    .devices
                    .iter()
                    .map(|d| {
                        // Mark the integrated device of this running instance to distinguish it
                        // from other `spotify-player` instances running elsewhere.
                        let name = if d.is_integrated {
                            format!("{} (integrated)", d.name)
                        } else {
                            d.name.clone()
                        };
                        (format!("{name} | {}", d.id), current_device_id == d.id)
                    })
                    .collect();

                let rect = render_list_popup(frame, rect, "Devices", items, 5, ui);
                (rect, false)
            }
            PopupState::ThemeList(themes, ..) => {
                let items = themes.iter().map(|t| (t.name.clone(), false)).collect();

                let rect = render_list_popup(frame, rect, "Themes", items, 7, ui);
                (rect, false)
            }
            PopupState::UserPlaylistList(action, _) => {
                let data = state.data.read();
                let (items, search_query) = match action {
                    PlaylistPopupAction::Browse {
                        folder_id,
                        search_query,
                    } => (
                        data.user_data.folder_playlists_items(*folder_id),
                        search_query,
                    ),
                    PlaylistPopupAction::AddTrack {
                        folder_id,
                        search_query,
                        ..
                    }
                    | PlaylistPopupAction::AddEpisode {
                        folder_id,
                        search_query,
                        ..
                    } => (
                        data.user_data.modifiable_playlist_items(Some(*folder_id)),
                        search_query,
                    ),
                };

                // Filter items based on search query if present
                let filtered_items = filtered_items_from_query(search_query, &items);

                let display_items = filtered_items
                    .iter()
                    .map(|p| (p.to_string(), false))
                    .collect();

                let chunks = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Fill(0),
                    Constraint::Length(10),
                ])
                .split(rect);

                // Render search input
                let search_rect = construct_and_render_block(
                    "Search Playlists (type to search, backspace on empty to close)",
                    &ui.theme,
                    Borders::ALL,
                    frame,
                    chunks[0],
                );
                frame.render_widget(Paragraph::new(format!("🔍 {search_query}")), search_rect);

                // Render filtered playlist list
                let rect =
                    render_list_popup(frame, chunks[2], "User Playlists", display_items, 10, ui);
                (rect, false)
            }
            PopupState::UserFollowedArtistList { .. } => {
                let items = state
                    .data
                    .read()
                    .user_data
                    .followed_artists
                    .iter()
                    .map(|a| (a.to_string(), false))
                    .collect();

                let rect = render_list_popup(frame, rect, "User Followed Artists", items, 7, ui);
                (rect, false)
            }
            PopupState::UserSavedAlbumList { .. } => {
                let items = state
                    .data
                    .read()
                    .user_data
                    .saved_albums
                    .iter()
                    .map(|a| (a.to_string(), false))
                    .collect();

                let rect = render_list_popup(frame, rect, "User Saved Albums", items, 7, ui);
                (rect, false)
            }
            PopupState::ArtistList(_, artists, ..) => {
                let items = artists.iter().map(|a| (a.to_string(), false)).collect();

                let rect = render_list_popup(frame, rect, "Artists", items, 5, ui);
                (rect, false)
            }
            PopupState::ConfirmAction { message, .. } => {
                let chunks =
                    Layout::vertical([Constraint::Fill(0), Constraint::Length(3)]).split(rect);

                let confirm_rect = construct_and_render_block(
                    "Confirm",
                    &ui.theme,
                    Borders::ALL,
                    frame,
                    chunks[1],
                );

                frame.render_widget(Paragraph::new(format!("{message} (y/n)")), confirm_rect);

                (chunks[0], true)
            }
        },
    }
}

/// A helper function to render a list popup
fn render_list_popup(
    frame: &mut Frame,
    rect: Rect,
    title: &str,
    items: Vec<(String, bool)>,
    length: u16,
    ui: &mut UIStateGuard,
) -> Rect {
    let chunks = Layout::vertical([Constraint::Fill(0), Constraint::Length(length)]).split(rect);

    let rect = construct_and_render_block(title, &ui.theme, Borders::ALL, frame, chunks[1]);
    let selected_index = ui.popup.as_ref().and_then(PopupState::list_selected);
    let (list, len) = utils::construct_list_widget(&ui.theme, items, true, selected_index);

    utils::render_list_window(
        frame,
        list,
        rect,
        len,
        ui.popup.as_mut().unwrap().list_state_mut().unwrap(),
    );

    chunks[0]
}

/// Same as `render_list_popup`, but anchored at a specific (column, row) -
/// i.e. wherever the user right-clicked - instead of always docking to the
/// bottom of `rect`. The popup is clamped to stay fully inside `rect` so it
/// never gets cut off by the edge of the terminal.
fn render_positioned_list_popup(
    frame: &mut Frame,
    rect: Rect,
    click_x: u16,
    click_y: u16,
    title: &str,
    items: Vec<(String, bool)>,
    height: u16,
    ui: &mut UIStateGuard,
) -> Rect {
    // Pick a reasonable width: wide enough for the title and the longest
    // item, capped so it doesn't dominate a small terminal.
    let content_width = items
        .iter()
        .map(|(s, _)| s.chars().count())
        .chain(std::iter::once(title.chars().count()))
        .max()
        .unwrap_or(20) as u16
        + 4; // borders + padding
    let width = content_width.clamp(20, rect.width.max(20));
    let height = height.min(rect.height.max(1));

    // Anchor the top-left corner at the click position, then clamp so the
    // whole popup stays within `rect`'s bounds.
    let x = click_x.min(rect.x + rect.width.saturating_sub(width));
    let y = click_y.min(rect.y + rect.height.saturating_sub(height));
    let x = x.max(rect.x);
    let y = y.max(rect.y);

    let popup_rect = Rect {
        x,
        y,
        width,
        height,
    };

    let inner_rect = construct_and_render_block(title, &ui.theme, Borders::ALL, frame, popup_rect);
    let selected_index = ui.popup.as_ref().and_then(PopupState::list_selected);
    let (list, len) = utils::construct_list_widget(&ui.theme, items, true, selected_index);

    utils::render_list_window(
        frame,
        list,
        inner_rect,
        len,
        ui.popup.as_mut().unwrap().list_state_mut().unwrap(),
    );

    // The rest of the screen (everything outside the small popup box) is what
    // the caller treats as "still visible main layout area" - for a
    // corner-anchored popup that's effectively the whole original rect, since
    // we don't want the page underneath to reflow around a floating box.
    rect
}

/// Render a shortcut help popup to show the available shortcuts based on user's inputs
pub fn render_shortcut_help_popup(frame: &mut Frame, ui: &mut UIStateGuard, rect: Rect) -> Rect {
    let input = &ui.input_key_sequence;

    // get the matches (keymaps) from the current key sequence input,
    // if there is at lease one match, render the shortcut help popup
    let matches = {
        if input.keys.is_empty() {
            vec![]
        } else {
            config::get_config()
                .keymap_config
                .find_matched_prefix_keymaps(input)
                .into_iter()
                .map(|keymap| {
                    let mut keymap = keymap.clone();
                    keymap.key_sequence.keys.drain(0..input.keys.len());
                    keymap
                })
                .filter(|keymap| {
                    !keymap.key_sequence.keys.is_empty() && keymap.command != Command::None
                })
                .collect::<Vec<_>>()
        }
    };

    if matches.is_empty() {
        rect
    } else {
        let chunks = Layout::vertical([Constraint::Fill(0), Constraint::Length(7)]).split(rect);

        let rect =
            construct_and_render_block("Shortcuts", &ui.theme, Borders::ALL, frame, chunks[1]);

        let help_table = Table::new(
            matches
                .into_iter()
                .map(|km| format!("{}: {:?}", km.key_sequence, km.command))
                .collect::<Vec<_>>()
                .chunks(SHORTCUT_TABLE_N_COLUMNS)
                .map(|c| Row::new(c.iter().map(|i| Cell::from(i.to_owned()))))
                .collect::<Vec<_>>(),
            SHORTCUT_TABLE_CONSTRAINS,
        );

        frame.render_widget(help_table, rect);
        chunks[0]
    }
}