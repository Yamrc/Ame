use std::rc::Rc;

use nekowg::{AnyElement, App, px};

use crate::component::cover_card::{self, CoverCardActions, PlaylistCoverCardProps};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistCardProps {
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
    pub cover_height: nekowg::Pixels,
}

impl PlaylistCardProps {
    pub fn standard(
        title: impl Into<String>,
        subtitle: impl Into<String>,
        cover_url: Option<String>,
    ) -> Self {
        Self {
            title: title.into(),
            subtitle: subtitle.into(),
            cover_url,
            cover_height: px(166.),
        }
    }
}

type CardAction = Rc<dyn Fn(&mut App)>;

#[derive(Clone, Default)]
pub struct PlaylistCardActions {
    pub on_open: Option<CardAction>,
}

pub fn render(props: PlaylistCardProps, actions: PlaylistCardActions) -> AnyElement {
    cover_card::render_playlist_card(
        PlaylistCoverCardProps {
            title: props.title,
            subtitle: props.subtitle,
            cover_url: props.cover_url,
            cover_height: props.cover_height,
        },
        CoverCardActions {
            on_open: actions.on_open,
        },
    )
}

pub fn subtitle_with_count(track_count: Option<u32>, creator: &str) -> String {
    match track_count {
        Some(track_count) => format!("{track_count} 首 · {creator}"),
        None => creator.to_string(),
    }
}
