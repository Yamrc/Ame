use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{sync::Arc, time::Duration};

use dashmap::DashMap;
use nekowg::*;
use parking_lot::RwLock;
use smol::channel::{self, Receiver, Sender};

use super::animation::{AnimationPriority, Event};
use super::interpolate::State;

pub mod color;
pub mod general;
pub mod position;

pub trait Transition: Send + Sync + 'static {
    fn run(&self, start: std::time::Instant, duration: std::time::Duration) -> f32 {
        let t = (start.elapsed().as_secs_f32() / duration.as_secs_f32()).min(1.0);
        self.calculate(t)
    }

    fn calculate(&self, t: f32) -> f32;
}

pub trait IntoArcTransition<T: Transition + 'static> {
    fn into_arc(self) -> Arc<T>;
}

impl<T: Transition + 'static> IntoArcTransition<T> for T {
    fn into_arc(self) -> Arc<T> {
        Arc::new(self)
    }
}

impl<T: Transition + 'static> IntoArcTransition<T> for Arc<T> {
    fn into_arc(self) -> Arc<T> {
        self
    }
}

pub(crate) struct PersistentContext {
    event: Event,
    style: StyleRefinement,
    duration: Duration,
    transition: Arc<dyn Transition>,
    priority: AnimationPriority,
}

pub(crate) struct ActiveAnimation {
    event: Event,
    duration: Duration,
    origin_duration: Duration,
    transition: Arc<dyn Transition>,
    ver: usize,
    persistent: bool,
}

pub(crate) struct TransitionRegistry {
    initialized: AtomicBool,
    rem_size: RwLock<Pixels>,
    states: DashMap<ElementId, State<StyleRefinement>>,
    active_animations: DashMap<ElementId, ActiveAnimation>,
    saved_contexts: DashMap<ElementId, PersistentContext>,
    wakeup_tx: Sender<()>,
    wakeup_rx: Receiver<()>,
}

pub(crate) static TRANSITION_REGISTRY: LazyLock<TransitionRegistry> = LazyLock::new(|| {
    let (tx, rx) = channel::unbounded();

    TransitionRegistry {
        initialized: AtomicBool::new(false),
        rem_size: RwLock::new(Pixels::from(16.)),
        states: Default::default(),
        active_animations: Default::default(),
        saved_contexts: Default::default(),
        wakeup_tx: tx,
        wakeup_rx: rx,
    }
});

impl TransitionRegistry {
    pub fn init(window: &mut Window, cx: &mut App) {
        if !TRANSITION_REGISTRY.initialized.swap(true, Ordering::SeqCst) {
            *TRANSITION_REGISTRY.rem_size.write() = window.rem_size();
            cx.spawn(Self::animation_tick).detach();
        }
    }

    pub fn rem_size() -> Pixels {
        *TRANSITION_REGISTRY.rem_size.read()
    }

    pub fn save_persistent_context(
        id: &ElementId,
        style: &StyleRefinement,
        duration: Duration,
        transition: Arc<dyn Transition>,
        priority: AnimationPriority,
    ) {
        if let Some(active_anim) = TRANSITION_REGISTRY.active_animations.get(id)
            && active_anim.persistent
        {
            TRANSITION_REGISTRY.saved_contexts.insert(
                id.clone(),
                PersistentContext {
                    event: active_anim.event.clone(),
                    style: style.clone(),
                    duration,
                    transition,
                    priority,
                },
            );
        }
    }

    pub fn remove_persistent_context(id: &ElementId, event: Event) {
        TRANSITION_REGISTRY
            .saved_contexts
            .remove_if(id, |_, ctx| ctx.event.eq(&event));
    }

    pub fn background_animated_task(
        id: ElementId,
        event: Event,
        duration: Duration,
        origin_duration: Duration,
        transition: Arc<dyn Transition>,
        ver: usize,
        persistent: bool,
    ) {
        TRANSITION_REGISTRY.active_animations.insert(
            id,
            ActiveAnimation {
                event,
                duration,
                origin_duration,
                transition,
                ver,
                persistent,
            },
        );

        TRANSITION_REGISTRY.wakeup_tx.try_send(()).ok();
    }

    pub async fn animation_tick(cx: &mut AsyncApp) {
        // least 120TPS
        let frame_duration = Duration::from_secs_f32(1. / 120.);
        let registry = &*TRANSITION_REGISTRY;

        loop {
            let mut changed = false;
            let removed = DashMap::new();

            {
                registry.active_animations.retain(|id, active| {
                    if let Some(mut state) = registry.states.get_mut(id) {
                        changed = true;

                        if state.animated(
                            active.ver,
                            active.duration,
                            &active.transition,
                            active.persistent,
                        ) {
                            if active.event.ne(&Event::None) {
                                state.priority = AnimationPriority::Lowest;
                                removed.insert(
                                    id.clone(),
                                    (active.origin_duration, active.transition.clone()),
                                );
                            }

                            false
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                });
            }

            if changed {
                cx.update(|cx| cx.refresh_windows());
            }

            registry.states.iter_mut().for_each(|mut state| {
                if state.progress >= 1. {
                    if let Some((id, ctx)) = registry.saved_contexts.remove(state.key()) {
                        state.priority = ctx.priority;
                        state.to = ctx.style;

                        let (ver, dt) = state.pre_animated(ctx.duration);

                        Self::background_animated_task(
                            id.clone(),
                            ctx.event.clone(),
                            dt,
                            dt,
                            ctx.transition.clone(),
                            ver,
                            true,
                        );
                    } else if let Some(active_anim) = registry.active_animations.get(state.key())
                        && !active_anim.persistent
                    {
                        state.priority = AnimationPriority::Lowest;

                        let mut fallback = None;
                        if let Some((_, cur_anim)) = removed.remove(state.key()) {
                            let (ver, dt) = state.pre_animated(cur_anim.0);
                            fallback = Some((ver, dt, cur_anim.1));
                        }

                        if let Some((ver, dt, transition)) = fallback {
                            state.to = state.origin.clone();

                            Self::background_animated_task(
                                state.key().clone(),
                                Event::None,
                                dt,
                                dt,
                                transition,
                                ver,
                                false,
                            );
                        }
                    }
                }
            });

            if registry.active_animations.is_empty() {
                registry.wakeup_rx.recv().await.ok();
            } else {
                smol::Timer::after(frame_duration).await;
            }
        }
    }

    #[inline]
    pub fn with_state_default<R>(
        id: ElementId,
        default: &StyleRefinement,
        f: impl FnOnce(&mut State<StyleRefinement>) -> R,
    ) -> R {
        let mut state = TRANSITION_REGISTRY
            .states
            .entry(id)
            .or_insert_with(|| State::new(default.clone()));

        f(&mut state)
    }

    #[inline]
    pub fn state_mut(
        id: ElementId,
    ) -> Option<dashmap::mapref::one::RefMut<'static, ElementId, State<StyleRefinement>>> {
        if !TRANSITION_REGISTRY
            .initialized
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return None;
        }

        TRANSITION_REGISTRY.states.get_mut(&id)
    }
}
