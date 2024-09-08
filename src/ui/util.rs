use ahash::AHashMap;
use gpui::{AppContext, Model, Render, View, WindowContext};
use tracing::debug;

pub fn prune_views<T>(
    views_model: Model<AHashMap<usize, View<T>>>,
    render_counter: Model<usize>,
    current: usize,
    cx: &mut AppContext,
) where
    T: Render,
{
    let last = *render_counter.read(cx);
    let mut to_remove: Vec<usize> = Vec::new();

    // determine whether or not we are at the start of a new render cycle
    if current < last {
        // we are at the start of a new render cycle
        // prune views that are no longer in the bounds (current..last)
        for (idx, _) in views_model.read(cx).iter() {
            if *idx < current || *idx >= (last + 1) {
                to_remove.push(*idx);
            }
        }
    }

    for idx in to_remove {
        views_model.update(cx, |m, _| {
            debug!("Removing view at index: {}", idx);
            m.remove(&idx);
        });
    }

    // update the render counter
    render_counter.update(cx, |m, _| {
        *m = current;
    });
}

pub fn create_or_retrieve_view<T>(
    views_model: Model<AHashMap<usize, View<T>>>,
    idx: usize,
    creation_fn: impl FnOnce(&mut WindowContext<'_>) -> View<T>,
    cx: &mut WindowContext<'_>,
) -> View<T>
where
    T: Render,
{
    let view = views_model.read(cx).get(&idx).cloned();
    match view {
        Some(view) => view,
        None => {
            let view = creation_fn(cx);
            views_model.update(cx, |m, _| {
                m.insert(idx, view.clone());
            });
            view
        }
    }
}
