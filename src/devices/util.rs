use intx::{I24, U24};

use crate::media::playback::{GetInnerSamples, Samples};

pub fn interleave<T>(samples: Vec<Vec<T>>) -> Vec<T>
where
    T: Copy + PartialEq,
{
    if samples.is_empty() {
        return vec![];
    }

    let length = samples.len();
    let mut result = vec![];

    for i in 0..(samples.len() * samples[0].len()) {
        result.push(samples[i % length][i / length]);
    }

    result
}

pub trait Packed {
    fn pack(&self) -> Vec<u8>;
}

macro_rules! impl_packed {
    ($t:ty) => {
        impl Packed for [$t] {
            fn pack(&self) -> Vec<u8> {
                self.iter()
                    .flat_map(|&x| x.to_ne_bytes().to_vec())
                    .collect()
            }
        }
    };
}

impl_packed!(u16);
impl_packed!(U24);
impl_packed!(u32);
impl_packed!(i16);
impl_packed!(I24);
impl_packed!(i32);
impl_packed!(i8);
impl_packed!(f32);
impl_packed!(f64);

// special cases
impl Packed for [u8] {
    fn pack(&self) -> Vec<u8> {
        self.to_vec()
    }
}
