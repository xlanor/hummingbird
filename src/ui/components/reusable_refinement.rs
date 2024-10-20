use gpui::{Div, InteractiveElement, StatefulInteractiveElement, StyleRefinement, Styled};

pub trait ReusableRefinement {
    /// Refine the style for the base state.
    ///
    /// GPUI supports modifying the base state, so the destination is passed in as a mutable
    /// reference. This allows the Refinement to modify the base state directly.
    fn base(&self, dest: &mut StyleRefinement) {
        // do nothing
    }
    /// Refine the style for the hover state.
    ///
    /// Note that GPUI does not currently allow for the hover state to ever be modified, only set.
    /// If your Refinement specifies hover styles, they will replace the previously set hover
    /// styles.
    fn hover(&self) -> Option<StyleRefinement> {
        None
    }
    /// Refine the style for the active state.
    ///
    /// Note that GPUI does not currently allow for the active state to ever be modified, only set.
    /// If your Refinement specifies active styles, they will replace the previously set active
    /// styles.
    fn active(&self) -> Option<StyleRefinement> {
        None
    }
}

// can't be called refinable because it's already a trait from GPUI
pub trait Refined {
    fn refine(self, refinement: &impl ReusableRefinement) -> Self;
}

impl<T> Refined for T
where
    T: InteractiveElement,
{
    fn refine(mut self, refinement: &impl ReusableRefinement) -> Self {
        refinement.base(&mut self.interactivity().base_style);

        if let Some(hover) = refinement.hover() {
            self.hover(|_| hover)
        } else {
            self
        }
    }
}

// this is required because of ambiguity between InteractiveElement and StatefulInteractiveElement
// i really dont see why, if StatefulInteractiveElement is a superset of InteractiveElement, the
// compiler could not just be written to prefer the implementation for StatefulInteractiveElement
// over the implementation for InteractiveElement, but it doesn't work that way so there's two
// traits
pub trait RefinedStateful {
    fn refine_with_active(self, refinement: &impl ReusableRefinement) -> Self;
}

impl<T> RefinedStateful for T
where
    T: StatefulInteractiveElement,
{
    fn refine_with_active(mut self, refinement: &impl ReusableRefinement) -> Self {
        refinement.base(&mut self.interactivity().base_style);

        let this = if let Some(hover) = refinement.hover() {
            self.hover(|_| hover)
        } else {
            self
        };

        if let Some(active) = refinement.active() {
            this.active(|_| active)
        } else {
            this
        }
    }
}
