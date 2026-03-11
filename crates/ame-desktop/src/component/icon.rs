use nekowg::{IntoElement, prelude::*, px, rgb, svg};

#[derive(Clone, Copy, Debug)]
pub enum IconName {
    ArrowLeft,
    ArrowRight,
    Fm,
    Previous,
    Play,
    Pause,
    Next,
    List,
    Repeat,
    RepeatOne,
    Shuffle,
    ThumbsDown,
    Volume,
    // Minus,
    // Plus,
    WindowMinimize,
    WindowMaximize,
    WindowRestore,
    WindowClose,
}

pub fn render(icon: IconName, size: f32, color: u32) -> impl IntoElement {
    svg()
        .path(path_for(icon))
        .size(px(size))
        .text_color(rgb(color))
}

fn path_for(icon: IconName) -> &'static str {
    match icon {
        IconName::ArrowLeft => "icon/font-awesome/arrow-left.svg",
        IconName::ArrowRight => "icon/font-awesome/arrow-right.svg",
        IconName::Fm => "icon/font-awesome/fm.svg",
        IconName::Previous => "icon/font-awesome/previous.svg",
        IconName::Play => "icon/font-awesome/play.svg",
        IconName::Pause => "icon/font-awesome/pause.svg",
        IconName::Next => "icon/font-awesome/next.svg",
        IconName::List => "icon/font-awesome/list.svg",
        IconName::Repeat => "icon/font-awesome/repeat.svg",
        IconName::RepeatOne => "icon/font-awesome/repeat-1.svg",
        IconName::Shuffle => "icon/font-awesome/shuffle.svg",
        IconName::ThumbsDown => "icon/font-awesome/thumbs-down.svg",
        IconName::Volume => "icon/font-awesome/volume.svg",
        // IconName::Minus => "icon/font-awesome/minus.svg",
        // IconName::Plus => "icon/font-awesome/plus.svg",
        IconName::WindowMinimize => "icon/vscode-codicons/window-minimize.svg",
        IconName::WindowMaximize => "icon/vscode-codicons/window-maximize.svg",
        IconName::WindowRestore => "icon/vscode-codicons/window-restore.svg",
        IconName::WindowClose => "icon/vscode-codicons/window-close.svg",
    }
}
