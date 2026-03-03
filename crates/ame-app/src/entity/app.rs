#[derive(Debug, Clone, Default)]
pub struct AppEntity {
    pub route: String,
}

impl AppEntity {
    pub fn navigate(&mut self, route: impl Into<String>) {
        self.route = route.into();
    }
}
