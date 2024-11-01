use gpui::*;
use smallvec::SmallVec;
use std::{
    cell::RefCell,
    iter,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
use tracing::info;

pub struct ContextMenu {
    pub(self) id: ElementId,
    pub(self) style: StyleRefinement,
    pub(self) element: Option<AnyElement>,
}

impl ContextMenu {
    pub fn with(mut self, element: impl IntoElement) -> Self {
        self.element = Some(element.into_any_element());
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for ContextMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ContextMenu {
    type RequestLayoutState = ();

    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style::default();

        let mut layout_ids: SmallVec<[LayoutId; 2]> = SmallVec::new();

        if let Some(element) = self.element.as_mut() {
            layout_ids.push(element.request_layout(cx));
        }

        (cx.request_layout(style, layout_ids), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        if let Some(element) = self.element.as_mut() {
            element.prepaint(cx);
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        if let Some(element) = self.element.as_mut() {
            element.paint(cx);
        }

        cx.with_element_state(id.unwrap(), |prev: Option<Rc<AtomicBool>>, cx| {
            let bool = prev.unwrap_or_else(|| {
                info!("creating");
                Rc::new(AtomicBool::new(false))
            });
            let bool_clone = bool.clone();

            cx.on_mouse_event(move |ev: &MouseDownEvent, phase, cx| {
                if ev.button == MouseButton::Right
                    && phase == DispatchPhase::Bubble
                    && bounds.contains(&ev.position)
                {
                    bool_clone.store(true, Ordering::Release);
                }
            });

            let bool_clone_2 = bool.clone();

            cx.on_mouse_event(move |ev: &MouseUpEvent, phase, cx| {
                if phase == DispatchPhase::Bubble
                    && ev.button == MouseButton::Right
                    && bounds.contains(&ev.position)
                    && bool_clone_2.swap(false, Ordering::AcqRel)
                {
                    info!("clicked");
                } else if phase == DispatchPhase::Bubble {
                    bool_clone_2.store(false, Ordering::Release)
                }
            });

            ((), bool)
        })
    }
}

pub fn context(id: impl Into<ElementId>) -> ContextMenu {
    ContextMenu {
        id: id.into(),
        style: StyleRefinement::default(),
        element: None,
    }
}
