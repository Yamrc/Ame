use std::rc::Rc;

use nekowg::{AnyElement, App, MouseButton, div, prelude::*, rgb};

use crate::component::{button, theme};

pub(crate) type OptionAction = Rc<dyn Fn(&mut App)>;

pub(crate) fn setting_option_row(
    label: impl Into<String>,
    actions: Vec<(&'static str, OptionAction)>,
) -> AnyElement {
    let actions = actions.into_iter().fold(
        div().flex().items_center().gap_2(),
        |row, (text, on_click)| {
            row.child(
                button::pill_base(text).on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_click(cx);
                }),
            )
        },
    );

    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .text_color(rgb(theme::COLOR_SECONDARY))
        .child(label.into())
        .child(actions)
        .into_any_element()
}
