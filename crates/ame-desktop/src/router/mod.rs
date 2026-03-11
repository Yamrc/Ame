use std::collections::HashMap;

use nekowg::{
    AnyElement, App, AppContext, BorrowAppContext, Context, Empty, Global, IntoElement,
    SharedString,
};

pub type RouteParams = HashMap<SharedString, SharedString>;

#[derive(Debug, Clone)]
pub struct Location {
    pub pathname: SharedString,
}

#[derive(Debug, Clone)]
pub struct RouterState {
    pub location: Location,
    pub params: RouteParams,
}

impl Default for RouterState {
    fn default() -> Self {
        Self {
            location: Location {
                pathname: "/".into(),
            },
            params: RouteParams::default(),
        }
    }
}

impl Global for RouterState {}

pub fn init(cx: &mut App) {
    cx.update_default_global(|state: &mut RouterState, _| {
        if state.location.pathname.is_empty() {
            state.location.pathname = "/".into();
        }
    });
}

pub fn current_path<T>(cx: &mut Context<T>) -> SharedString {
    cx.read_global(|state: &RouterState, _| state.location.pathname.clone())
}

pub fn use_params<T>(cx: &mut Context<T>) -> RouteParams {
    cx.read_global(|state: &RouterState, _| state.params.clone())
}

pub fn navigate<T>(cx: &mut Context<T>, path: impl Into<SharedString>) {
    let normalized = normalize_path(&path.into());
    cx.update_global(|state: &mut RouterState, _| {
        state.location.pathname = normalized;
        state.params.clear();
    });
}

pub struct Route<T> {
    path: Option<SharedString>,
    index: bool,
    element: Option<Box<dyn Fn(&RouteParams, &mut Context<T>) -> AnyElement>>,
}

impl<T> Route<T> {
    pub fn new() -> Self {
        Self {
            path: None,
            index: false,
            element: None,
        }
    }

    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn index(mut self) -> Self {
        self.index = true;
        self
    }

    pub fn element<F>(mut self, f: F) -> Self
    where
        F: Fn(&RouteParams, &mut Context<T>) -> AnyElement + 'static,
    {
        self.element = Some(Box::new(f));
        self
    }
}

pub struct Routes<T> {
    basename: SharedString,
    routes: Vec<Route<T>>,
}

impl<T> Routes<T> {
    pub fn new() -> Self {
        Self {
            basename: "/".into(),
            routes: Vec::new(),
        }
    }

    pub fn basename(mut self, base: impl Into<SharedString>) -> Self {
        self.basename = base.into();
        self
    }

    pub fn child(mut self, route: Route<T>) -> Self {
        self.routes.push(route);
        self
    }

    pub fn render(self, cx: &mut Context<T>) -> AnyElement {
        let pathname = cx.read_global(|state: &RouterState, _| state.location.pathname.clone());
        let relative = strip_basename(&pathname, &self.basename);
        let relative = relative.trim_matches('/');

        for route in self.routes {
            if route.index && relative.is_empty() {
                return render_route(route, RouteParams::default(), cx);
            }

            if let Some(route_path) = route.path.as_ref() {
                if let Some(params) = match_route(route_path, relative) {
                    return render_route(route, params, cx);
                }
            }
        }

        Empty.into_any_element()
    }
}

fn render_route<T>(route: Route<T>, params: RouteParams, cx: &mut Context<T>) -> AnyElement {
    cx.update_global(|state: &mut RouterState, _| {
        state.params = params.clone();
    });
    let Some(element) = route.element else {
        return Empty.into_any_element();
    };
    element(&params, cx)
}

fn strip_basename(path: &SharedString, basename: &SharedString) -> String {
    let path = normalize_path(path);
    let base = normalize_path(basename);
    if base == "/" {
        return path.as_ref().to_string();
    }
    let path_str = path.as_ref();
    let base_str = base.as_ref();
    if let Some(rest) = path_str.strip_prefix(base_str) {
        let trimmed = rest.trim_start_matches('/');
        if trimmed.is_empty() {
            "/".to_string()
        } else {
            format!("/{trimmed}")
        }
    } else {
        path_str.to_string()
    }
}

fn match_route(pattern: &SharedString, path: &str) -> Option<RouteParams> {
    let pattern = pattern.trim_matches('/');
    let path = path.trim_matches('/');

    let pattern_segments = if pattern.is_empty() {
        Vec::new()
    } else {
        pattern.split('/').collect::<Vec<_>>()
    };
    let path_segments = if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').collect::<Vec<_>>()
    };

    if pattern_segments.len() != path_segments.len() {
        return None;
    }

    let mut params = RouteParams::default();
    for (pattern_segment, path_segment) in pattern_segments.iter().zip(path_segments.iter()) {
        if let Some(key) = pattern_segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        {
            if !key.is_empty() {
                params.insert(
                    SharedString::from(key.to_string()),
                    SharedString::from(path_segment.to_string()),
                );
            }
        } else if pattern_segment != path_segment {
            return None;
        }
    }

    Some(params)
}

fn normalize_path(path: &SharedString) -> SharedString {
    let mut value = path.as_ref().trim().to_string();
    if value.is_empty() {
        value = "/".to_string();
    }
    if !value.starts_with('/') {
        value.insert(0, '/');
    }
    if value.len() > 1 && value.ends_with('/') {
        value.pop();
    }
    SharedString::from(value)
}
