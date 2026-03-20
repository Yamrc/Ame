use std::rc::Rc;

use nekowg::{App, SharedString, Window};

use crate::component::common::context_menu::style::{ContextMenuHeader, ContextMenuTone};
use crate::component::icon::IconName;

use super::ContextMenuContent;

type ContextMenuAction = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
pub enum ContextMenuItem {
    Item {
        label: SharedString,
        icon: Option<IconName>,
        shortcut: Option<SharedString>,
        tone: ContextMenuTone,
        disabled: bool,
        action: Option<ContextMenuAction>,
    },
    Separator,
}

pub struct ContextMenuBuilder {
    header: Option<ContextMenuHeader>,
    items: Vec<ContextMenuItem>,
}

impl ContextMenuBuilder {
    pub fn new() -> Self {
        Self {
            header: None,
            items: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn header(mut self, header: ContextMenuHeader) -> Self {
        self.header = Some(header);
        self
    }

    pub fn track_header(
        mut self,
        cover_url: Option<impl Into<SharedString>>,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Self {
        self.header = Some(ContextMenuHeader::Track {
            cover_url: cover_url.map(Into::into),
            title: title.into(),
            subtitle: subtitle.into(),
        });
        self
    }

    #[allow(dead_code)]
    pub fn text_header(
        mut self,
        title: impl Into<SharedString>,
        subtitle: Option<impl Into<SharedString>>,
    ) -> Self {
        self.header = Some(ContextMenuHeader::Text {
            title: title.into(),
            subtitle: subtitle.map(Into::into),
        });
        self
    }

    #[allow(dead_code)]
    pub fn item_with(
        mut self,
        label: impl Into<SharedString>,
        icon: Option<IconName>,
        shortcut: Option<impl Into<SharedString>>,
        tone: ContextMenuTone,
        disabled: bool,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon,
            shortcut: shortcut.map(Into::into),
            tone,
            disabled,
            action: Some(Rc::new(action)),
        });
        self
    }

    pub fn item(
        mut self,
        label: impl Into<SharedString>,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon: None,
            shortcut: None,
            tone: ContextMenuTone::Normal,
            disabled: false,
            action: Some(Rc::new(action)),
        });
        self
    }

    #[allow(dead_code)]
    pub fn item_disabled(
        mut self,
        label: impl Into<SharedString>,
        disabled: bool,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon: None,
            shortcut: None,
            tone: ContextMenuTone::Normal,
            disabled,
            action: Some(Rc::new(action)),
        });
        self
    }

    #[allow(dead_code)]
    pub fn item_with_icon(
        mut self,
        label: impl Into<SharedString>,
        icon: IconName,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon: Some(icon),
            shortcut: None,
            tone: ContextMenuTone::Normal,
            disabled: false,
            action: Some(Rc::new(action)),
        });
        self
    }

    #[allow(dead_code)]
    pub fn item_shortcut(
        mut self,
        label: impl Into<SharedString>,
        shortcut: impl Into<SharedString>,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon: None,
            shortcut: Some(shortcut.into()),
            tone: ContextMenuTone::Normal,
            disabled: false,
            action: Some(Rc::new(action)),
        });
        self
    }

    #[allow(dead_code)]
    pub fn item_tone(
        mut self,
        label: impl Into<SharedString>,
        tone: ContextMenuTone,
        action: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(ContextMenuItem::Item {
            label: label.into(),
            icon: None,
            shortcut: None,
            tone,
            disabled: false,
            action: Some(Rc::new(action)),
        });
        self
    }

    #[allow(dead_code)]
    pub fn separator(mut self) -> Self {
        self.items.push(ContextMenuItem::Separator);
        self
    }

    pub(super) fn build(mut self) -> ContextMenuContent {
        normalize_items(&mut self.items);
        ContextMenuContent {
            header: self.header,
            items: self.items,
        }
    }
}

fn normalize_items(items: &mut Vec<ContextMenuItem>) {
    while matches!(items.first(), Some(ContextMenuItem::Separator)) {
        items.remove(0);
    }
    while matches!(items.last(), Some(ContextMenuItem::Separator)) {
        items.pop();
    }

    let mut last_was_separator = false;
    items.retain(|item| {
        let is_separator = matches!(item, ContextMenuItem::Separator);
        if is_separator && last_was_separator {
            return false;
        }
        last_was_separator = is_separator;
        true
    });
}
