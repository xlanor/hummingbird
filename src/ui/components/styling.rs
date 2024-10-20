use gpui::{Fill, Styled};

pub trait AdditionalStyleUtil {
    fn background_opacity(self, opacity: f32) -> Self;
}

impl<T> AdditionalStyleUtil for T
where
    T: Styled,
{
    fn background_opacity(mut self, opacity: f32) -> Self {
        match &mut self.style().background {
            Some(v) => match v {
                Fill::Color(hsla) => hsla.a = opacity,
            },
            None => (),
        }

        self
    }
}
