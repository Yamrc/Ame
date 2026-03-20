use nekowg::Context;

pub trait PageLifecycle: Sized {
    fn on_activate(&mut self, _cx: &mut Context<Self>) {}

    fn on_frozen(&mut self, _cx: &mut Context<Self>) {}

    fn on_destroy(&mut self, _cx: &mut Context<Self>) {}
}
