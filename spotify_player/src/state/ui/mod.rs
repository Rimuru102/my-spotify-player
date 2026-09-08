use crate::{
    config::{self, Theme},
    key,
    ui::{self, Orientation},
    utils::filtered_items_from_query,
};

#[cfg(feature = "image")]
use crate::ui::cover_image::CoverImage;
#[cfg(feature = "image")]
use ratatui_image::picker::Picker;

pub type UIStateGuard<'a> = parking_lot::MutexGuard<'a, UIState>;

mod page;
mod popup;

pub use page::*;
pub use popup::*;

#[cfg(feature = "image")]
#[derive(Default)]
pub struct ImageRenderInfo {
    pub url: String,
    pub render_area: ratatui::layout::Rect,
    pub state: Option<CoverImage>,
}

#[cfg(feature = "image")]
impl std::fmt::Debug for ImageRenderInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageRenderInfo")
            .field("url", &self.url)
            .field("render_area", &self.render_area)
            .field("state", &self.state.is_some())
            .finish()
    }
}

/// The rect of the currently-focused list/table widget, plus how many rows
/// at the top of that rect are a header row (1 for `Table` widgets that use
/// `.header(...)`, 0 for plain `List` widgets) rather than actual items.
/// Used to translate a mouse click into an exact item index.
#[derive(Debug, Clone, Copy)]
pub struct ActiveListRect {
    pub rect: ratatui::layout::Rect,
    pub header_rows: u16,
}

/// Application's UI state
#[derive(Debug)]
pub struct UIState {
    pub is_running: bool,
    pub theme: config::Theme,
    pub input_key_sequence: key::KeySequence,
    pub orientation: ui::Orientation,

    pub history: Vec<PageState>,
    pub popup: Option<PopupState>,

    /// The rectangle representing the playback progress bar,
    /// which is mainly used to handle mouse click events (for seeking command)
    pub playback_progress_bar_rect: ratatui::layout::Rect,
    /// The main content rect (below/above the playback window, whichever way
    /// it's docked), recomputed every frame. Coarse fallback for mouse hit-testing.
    pub content_area_rect: ratatui::layout::Rect,
    /// The exact rect of whichever list/table widget is currently focused,
    /// captured at the point it's actually rendered (inside borders, below
    /// any header row/description line). `None` if the current page has no
    /// focusable list (e.g. Lyrics page). Reset every frame before rendering,
    /// so a stale rect from a previous page can't leak into hit-testing.
    pub active_list_rect: Option<ActiveListRect>,
    /// Terminal coordinates of the last right-click, if a popup opened because
    /// of it. Lets the popup render anchored at the click instead of always
    /// docking to the bottom of the screen. Cleared once the popup closes.
    pub mouse_popup_anchor: Option<(u16, u16)>,

    /// Count prefix for vim-style navigation (e.g., 5j, 10k)
    pub count_prefix: Option<usize>,

    #[cfg(feature = "image")]
    pub last_cover_image_render_info: ImageRenderInfo,

    #[cfg(feature = "image")]
    pub picker: Picker,
}

impl UIState {
    pub fn current_page(&self) -> &PageState {
        self.history.last().expect("non-empty history")
    }

    pub fn current_page_mut(&mut self) -> &mut PageState {
        self.history.last_mut().expect("non-empty history")
    }

    pub fn new_search_popup(&mut self) {
        self.current_page_mut().select(0);
        self.popup = Some(PopupState::Search {
            query: String::new(),
        });
    }

    pub fn new_page(&mut self, page: PageState) {
        self.popup = None;
        if let Some(current_page) = self.history.last() {
            if &page == current_page {
                return;
            }
        }
        self.history.push(page);
    }

    /// Switch to a page, reusing an existing instance of the same `PageType`
    /// already in history (moving it to the top) instead of always pushing a
    /// fresh one. Intended for "singleton" pages that don't carry a unique ID
    /// - Library, Search, Browse, Queue, CommandHelp, Logs - so jumping back
    /// and forth (e.g. via F-key shortcuts) behaves like switching tabs:
    /// bounded history instead of growing forever, and scroll
    /// position/focus is preserved rather than reset every time.
    ///
    /// `Context` pages are intentionally NOT switched this way: they're
    /// parameterized by a specific playlist/album/artist id, and drilling
    /// from one into another is the normal, desired way `history` grows.
    ///
    /// Returns `true` if a fresh page was created (so the caller knows
    /// whether it needs to kick off a data-fetch request), `false` if an
    /// existing page was reused.
    pub fn switch_to_page(
        &mut self,
        page_type: PageType,
        make_default: impl FnOnce() -> PageState,
    ) -> bool {
        self.popup = None;
        if let Some(pos) = self.history.iter().position(|p| p.page_type() == page_type) {
            if pos != self.history.len() - 1 {
                let page = self.history.remove(pos);
                self.history.push(page);
            }
            false
        } else {
            self.history.push(make_default());
            true
        }
    }

    /// Return whether there exists a focused popup.
    ///
    /// Currently, only search popup is not focused when it's opened.
    pub fn has_focused_popup(&self) -> bool {
        match self.popup.as_ref() {
            None => false,
            Some(popup) => !matches!(popup, PopupState::Search { .. }),
        }
    }

    /// Get a list of items possibly filtered by a search query if exists a search popup
    pub fn search_filtered_items<'a, T: std::fmt::Display>(&self, items: &'a [T]) -> Vec<&'a T> {
        match self.popup {
            Some(PopupState::Search { ref query }) => filtered_items_from_query(query, items),
            _ => items.iter().collect::<Vec<_>>(),
        }
    }
}

use ratatui::layout::Rect;

impl Default for UIState {
    fn default() -> Self {
        Self {
            is_running: true,
            theme: Theme::default(),
            input_key_sequence: key::KeySequence { keys: vec![] },
            orientation: match crossterm::terminal::size() {
                Ok((columns, rows)) => ui::Orientation::from_size(columns, rows),
                Err(err) => {
                    tracing::warn!("Unable to get terminal size, error: {err:#}");
                    Orientation::default()
                }
            },

            history: vec![PageState::Library {
                state: LibraryPageUIState::new(),
            }],
            popup: None,

            playback_progress_bar_rect: Rect::default(),
            content_area_rect: Rect::default(),
            active_list_rect: None,
            mouse_popup_anchor: None,

            count_prefix: None,

            #[cfg(feature = "image")]
            last_cover_image_render_info: ImageRenderInfo::default(),

            // Will be reinitialize later in ui/mod.rs after init_ui()
            #[cfg(feature = "image")]
            picker: Picker::halfblocks(),
        }
    }
}