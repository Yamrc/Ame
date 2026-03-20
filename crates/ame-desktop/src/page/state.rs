use nekowg::{Context, Entity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Guest,
    User,
}

#[derive(Debug, Clone)]
pub struct DataState<T> {
    pub data: T,
    pub loading: bool,
    pub error: Option<String>,
    pub fetched_at_ms: Option<u64>,
    pub source: DataSource,
}

impl<T: Default> Default for DataState<T> {
    fn default() -> Self {
        Self {
            data: T::default(),
            loading: false,
            error: None,
            fetched_at_ms: None,
            source: DataSource::Guest,
        }
    }
}

impl<T: Default> DataState<T> {
    pub fn begin(&mut self, source: DataSource) {
        self.loading = true;
        self.error = None;
        self.source = source;
    }

    pub fn succeed(&mut self, data: T, fetched_at_ms: Option<u64>) {
        self.data = data;
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = fetched_at_ms;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.loading = false;
        self.error = Some(error.into());
    }

    pub fn clear(&mut self) {
        self.data = T::default();
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = None;
    }
}

pub trait FreezablePageState {
    fn release_for_freeze(&mut self);
}

pub fn freeze_page_state<TView, TState>(state: &Entity<TState>, cx: &mut Context<TView>)
where
    TView: Sized,
    TState: FreezablePageState + 'static,
{
    state.update(cx, |state, cx| {
        state.release_for_freeze();
        cx.notify();
    });
}
