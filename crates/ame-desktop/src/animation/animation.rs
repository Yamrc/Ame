use std::{collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use nekowg::{prelude::FluentBuilder, *};

use super::{
    interpolate::State,
    transition::{IntoArcTransition, Transition, TransitionRegistry, general::Linear},
};

type HoverListener = Rc<dyn Fn(&bool, &mut Window, &mut App)>;
type ClickListener = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type StyleModifier<T> = Rc<dyn Fn(&T, State<StyleRefinement>) -> State<StyleRefinement>>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Event {
    None,
    Hover,
    Click,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnimationPriority {
    Lowest = 0,
    Low = 25,
    Medium = 50,
    High = 75,
    Realtime = 100,
}

#[derive(IntoElement)]
pub struct AnimatedWrapper<E>
where
    E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static,
{
    style: StyleRefinement,
    children: Vec<AnyElement>,
    id: ElementId,
    child: E,
    transitions: HashMap<Event, (Duration, Arc<dyn Transition>)>,
    on_hover: Option<HoverListener>,
    hover_modifier: Option<StyleModifier<bool>>,
    on_click: Option<ClickListener>,
    click_modifier: Option<StyleModifier<ClickEvent>>,
}

#[allow(dead_code)]
impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    AnimatedWrapper<E>
{
    pub fn transition_on_hover<T, I>(
        mut self,
        duration: Duration,
        transition: I,
        modifier: impl Fn(&bool, State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transitions
            .insert(Event::Hover, (duration, transition.into_arc()));
        self.hover_modifier = Some(Rc::new(modifier));

        self
    }

    pub fn transition_on_click<T, I>(
        mut self,
        duration: Duration,
        transition: I,
        modifier: impl Fn(&ClickEvent, State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transitions
            .insert(Event::Click, (duration, transition.into_arc()));
        self.click_modifier = Some(Rc::new(modifier));

        self
    }

    pub fn on_hover(mut self, listener: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(listener));

        self
    }

    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(listener));

        self
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when<T, I>(
        self,
        condition: bool,
        duration: Duration,
        transition: I,
        then: impl FnOnce(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transition_when_with_priority(
            condition,
            duration,
            transition,
            AnimationPriority::Lowest,
            then,
        )
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_with_priority<T, I>(
        self,
        condition: bool,
        duration: Duration,
        transition: I,
        priority: AnimationPriority,
        then: impl FnOnce(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        if condition {
            Self::animated_handle_without_event(
                self.id.clone(),
                then,
                (duration, transition.into_arc()),
                priority,
            );
        }

        self
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_else<T, I>(
        self,
        condition: bool,
        duration: Duration,
        transition: I,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
        else_fn: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transition_when_else_with_priority(
            condition,
            duration,
            transition,
            AnimationPriority::Lowest,
            then,
            else_fn,
        )
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_else_with_priority<T, I>(
        self,
        condition: bool,
        duration: Duration,
        transition: I,
        priority: AnimationPriority,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
        else_fn: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        if condition {
            Self::animated_handle_without_event(
                self.id.clone(),
                then,
                (duration, transition.into_arc()),
                priority,
            );
        } else {
            Self::animated_handle_without_event(
                self.id.clone(),
                else_fn,
                (duration, transition.into_arc()),
                priority,
            );
        }

        self
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_some<T, I, O>(
        self,
        option: Option<O>,
        duration: Duration,
        transition: I,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transition_when_some_with_priority(
            option,
            duration,
            transition,
            AnimationPriority::Lowest,
            then,
        )
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_some_with_priority<T, I, O>(
        self,
        option: Option<O>,
        duration: Duration,
        transition: I,
        priority: AnimationPriority,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        if option.is_some() {
            Self::animated_handle_without_event(
                self.id.clone(),
                then,
                (duration, transition.into_arc()),
                priority,
            );
        }

        self
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_none<T, I, O>(
        self,
        option: &Option<O>,
        duration: Duration,
        transition: I,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        self.transition_when_none_with_priority(
            option,
            duration,
            transition,
            AnimationPriority::Lowest,
            then,
        )
    }

    /**
     * Changes made via .when(), .when_else(), etc., do not automatically trigger the animation cycle. Unlike event-based listeners that hold and manage the App context, these declarative methods do not pass the context to the animation controller. You must manually invoke a refresh or re-render to start the transition.
     */
    pub fn transition_when_none_with_priority<T, I, O>(
        self,
        option: &Option<O>,
        duration: Duration,
        transition: I,
        priority: AnimationPriority,
        then: impl Fn(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
    ) -> Self
    where
        T: Transition + 'static,
        I: IntoArcTransition<T>,
    {
        if option.is_none() {
            Self::animated_handle_without_event(
                self.id.clone(),
                then,
                (duration, transition.into_arc()),
                priority,
            );
        }

        self
    }
}

impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    AnimatedWrapper<E>
{
    fn with_transition(mut child: E, id: impl Into<ElementId>) -> Self {
        Self {
            style: child.style().clone(),
            children: Vec::new(),
            id: id.into(),
            child,
            transitions: HashMap::new(),
            on_click: None,
            on_hover: None,
            hover_modifier: None,
            click_modifier: None,
        }
    }
}

impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    Styled for AnimatedWrapper<E>
{
    fn style(&mut self) -> &mut nekowg::StyleRefinement {
        &mut self.style
    }
}

impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    ParentElement for AnimatedWrapper<E>
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    RenderOnce for AnimatedWrapper<E>
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        TransitionRegistry::init(window, cx);

        let mut root = self.child;

        TransitionRegistry::with_state_default(self.id.clone(), &self.style, |state| {
            root.style().refine(&state.cur);
        });

        let id_for_hover = self.id.clone();
        let on_hover_cb = self.on_hover;
        let hover_mod = self.hover_modifier;
        let hover_transition = self
            .transitions
            .get(&Event::Hover)
            .cloned()
            .unwrap_or_else(|| (Duration::default(), Arc::new(Linear)));

        let id_for_click = self.id.clone();
        let on_click_cb = self.on_click;
        let click_mod = self.click_modifier;
        let click_transition = self
            .transitions
            .get(&Event::Click)
            .cloned()
            .unwrap_or_else(|| (Duration::default(), Arc::new(Linear)));
        let should_bind_hover = on_hover_cb.is_some() || hover_mod.is_some();
        let should_bind_click = on_click_cb.is_some() || click_mod.is_some();

        if should_bind_hover {
            root = root.on_hover(move |hovered, window, app| {
                if let Some(cb) = &on_hover_cb {
                    cb(hovered, window, app);
                }

                if *hovered {
                    Self::animated_handle_persistent(
                        hovered,
                        id_for_hover.clone(),
                        Event::Hover,
                        hover_mod.clone(),
                        hover_transition.clone(),
                        AnimationPriority::Medium,
                    );
                } else {
                    TransitionRegistry::remove_persistent_context(&id_for_hover, Event::Hover);
                    Self::animated_handle(
                        hovered,
                        id_for_hover.clone(),
                        Event::Hover,
                        hover_mod.clone(),
                        hover_transition.clone(),
                        AnimationPriority::High,
                        false,
                    );
                }
            });
        }

        if should_bind_click {
            root = root.on_click(move |event, window, app| {
                if let Some(cb) = &on_click_cb {
                    cb(event, window, app);
                }

                Self::animated_handle(
                    event,
                    id_for_click.clone(),
                    Event::Click,
                    click_mod.clone(),
                    click_transition.clone(),
                    AnimationPriority::High,
                    true,
                );
            });
        }

        root.children(self.children)
    }
}

impl<E: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    AnimatedWrapper<E>
{
    fn animated_handle<T>(
        data: &T,
        id: ElementId,
        event: Event,
        modifier: Option<StyleModifier<T>>,
        transition: (Duration, Arc<dyn Transition>),
        priority: AnimationPriority,
        save_persistent: bool,
    ) {
        let mut should_start_task = None;

        {
            if let Some(mut state) = TransitionRegistry::state_mut(id.clone()) {
                let state_snapshot = state.clone();

                if state.priority <= priority
                    && let Some(modifier) = modifier
                {
                    if save_persistent {
                        TransitionRegistry::save_persistent_context(
                            &id,
                            &state.to,
                            transition.0,
                            transition.1.clone(),
                            state.priority,
                        );
                    }

                    // instantaneous events like hoverless/click
                    state.priority = priority;
                    *state = modifier(data, state.clone());

                    if state_snapshot.ne(&*state) {
                        let (ver, dt) = state.pre_animated(transition.0);
                        should_start_task = Some((ver, dt));
                    } else {
                        state.priority = AnimationPriority::Lowest;
                    }
                }
            } else {
                should_start_task = None;
            }
        }

        if let Some((ver, dt)) = should_start_task {
            TransitionRegistry::background_animated_task(
                id,
                event,
                dt,
                transition.0,
                transition.1,
                ver,
                false,
            );
        }
    }

    fn animated_handle_persistent<T>(
        data: &T,
        id: ElementId,
        event: Event,
        modifier: Option<StyleModifier<T>>,
        transition: (Duration, Arc<dyn Transition>),
        priority: AnimationPriority,
    ) {
        let mut should_start_task = None;

        {
            if let Some(mut state) = TransitionRegistry::state_mut(id.clone()) {
                let state_snapshot = state.clone();

                if state.priority <= priority
                    && let Some(modifier) = modifier
                {
                    // allow overridden by medium/high/realtime
                    state.priority = priority;
                    *state = modifier(data, state.clone());

                    if state_snapshot.ne(&*state) {
                        let (ver, dt) = state.pre_animated(transition.0);
                        should_start_task = Some((ver, dt));
                    } else {
                        state.priority = AnimationPriority::Lowest;
                    }
                }
            } else {
                should_start_task = None;
            }
        }

        if let Some((ver, dt)) = should_start_task {
            TransitionRegistry::background_animated_task(
                id,
                event,
                dt,
                transition.0,
                transition.1,
                ver,
                true,
            );
        }
    }
    fn animated_handle_without_event(
        id: ElementId,
        modifier: impl FnOnce(State<StyleRefinement>) -> State<StyleRefinement> + 'static,
        transition: (Duration, Arc<dyn Transition>),
        priority: AnimationPriority,
    ) {
        let mut should_start_task = None;

        {
            if let Some(mut state) = TransitionRegistry::state_mut(id.clone()) {
                let state_snapshot = state.clone();

                if state.priority <= priority {
                    state.priority = priority;
                    *state = modifier(state.clone());

                    if state_snapshot.ne(&*state) {
                        let (ver, dt) = state.pre_animated(transition.0);
                        should_start_task = Some((ver, dt));
                    } else {
                        state.priority = AnimationPriority::Lowest;
                    }
                }
            } else {
                should_start_task = None;
            }
        }

        if let Some((ver, dt)) = should_start_task {
            TransitionRegistry::background_animated_task(
                id,
                Event::None,
                dt,
                transition.0,
                transition.1,
                ver,
                false,
            );
        }
    }
}

pub trait TransitionExt:
    IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static
{
    fn with_transition(self, id: impl Into<ElementId>) -> AnimatedWrapper<Self> {
        AnimatedWrapper::with_transition(self, id)
    }
}

impl<T: IntoElement + StatefulInteractiveElement + ParentElement + FluentBuilder + Styled + 'static>
    TransitionExt for T
{
}
