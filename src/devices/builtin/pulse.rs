use std::marker::PhantomData;

use intx::I24;
use libpulse_binding::{
    channelmap::Map,
    error::PAErr,
    sample::{Format, Spec},
    stream::Direction,
};
use libpulse_simple_binding::Simple;
use pulsectl::controllers::{types::DeviceInfo, DeviceControl, SinkController};

use crate::{
    devices::{
        errors::{
            FindError, InfoError, InitializationError, ListError, OpenError, ResetError,
            SubmissionError,
        },
        format::{BufferSize, ChannelSpec, FormatInfo, SampleFormat, SupportedFormat},
        traits::{Device, DeviceProvider, OutputStream},
        util::{interleave, Packed, Scale},
    },
    media::playback::GetInnerSamples,
    util::make_unknown_error_unwrap,
};

// The code for this is absolutely awful because PulseAudio is awful. I'm sorry.
// Perhaps one day I will feel masochistic enough to rewrite this to not use 3 different libraries,
// properly support pausing, and use the PulseAudio volume control API. But for now, I can't be
// bothered.

pub struct PulseProvider {
    controller: SinkController,
}

impl Default for PulseProvider {
    fn default() -> Self {
        Self {
            controller: SinkController::create().unwrap(),
        }
    }
}

impl DeviceProvider for PulseProvider {
    fn initialize(&mut self) -> Result<(), InitializationError> {
        Ok(())
    }

    fn get_devices(&mut self) -> Result<Vec<Box<dyn Device>>, ListError> {
        Ok(self
            .controller
            .list_devices()
            .map_err(|e| ListError::Unknown(e.to_string()))?
            .into_iter()
            .map(|d| Box::new(PulseDevice::from(d)) as Box<dyn Device>)
            .collect())
    }

    fn get_default_device(&mut self) -> Result<Box<dyn Device>, FindError> {
        Ok(Box::new(PulseDevice::from(
            self.controller
                .get_default_device()
                .map_err(|e| FindError::Unknown(e.to_string()))?,
        )) as Box<dyn Device>)
    }

    fn get_device_by_uid(&mut self, id: &str) -> Result<Box<dyn Device>, FindError> {
        Ok(Box::new(PulseDevice::from(
            self.controller
                .get_device_by_name(id)
                .map_err(|e| FindError::Unknown(e.to_string()))?,
        )) as Box<dyn Device>)
    }
}

struct PulseDevice {
    pub info: DeviceInfo,
}

impl From<DeviceInfo> for PulseDevice {
    fn from(info: DeviceInfo) -> Self {
        Self { info: info.clone() }
    }
}

fn get_sample_format(format: Format) -> SampleFormat {
    match format {
        Format::U8 => SampleFormat::Unsigned8,
        Format::ALaw => {
            panic!("Invalid sample format (alaw unsupported)")
        }
        Format::ULaw => {
            panic!("Invalid sample format (ulaw unsupported)")
        }
        Format::S16le => SampleFormat::Signed16,
        Format::S16be => SampleFormat::Signed16,
        Format::S32le => SampleFormat::Signed32,
        Format::S32be => SampleFormat::Signed32,
        Format::F32le => SampleFormat::Float32,
        Format::F32be => SampleFormat::Float32,
        Format::S24_32le => SampleFormat::Signed24,
        Format::S24_32be => SampleFormat::Signed24,
        Format::S24le => SampleFormat::Signed24Packed,
        Format::S24be => SampleFormat::Signed24Packed,
        Format::Invalid => {
            panic!("Invalid sample format (from pulse)")
        }
    }
}

fn channel_spec(channel_map: Map) -> ChannelSpec {
    // TODO: implement bitmask
    ChannelSpec::Count(channel_map.len() as u16)
}

fn pulse_spec(format: FormatInfo) -> Spec {
    Spec {
        format: match format.sample_type {
            SampleFormat::Float32 => Format::FLOAT32NE,
            SampleFormat::Signed32 => Format::S32NE,
            SampleFormat::Signed24 => Format::S24_32NE,
            SampleFormat::Signed24Packed => Format::S24NE,
            SampleFormat::Signed16 => Format::S16NE,
            SampleFormat::Unsigned8 => Format::U8,
            _ => unimplemented!(),
        },
        rate: format.sample_rate,
        channels: match format.channels {
            // TODO: count bitmask channels
            ChannelSpec::Bitmask(_) => todo!(),
            ChannelSpec::Count(v) => v as u8,
        },
    }
}

impl Device for PulseDevice {
    fn open_device(&mut self, format: FormatInfo) -> Result<Box<dyn OutputStream>, OpenError> {
        let spec = pulse_spec(format.clone());
        assert!(spec.is_valid());

        let stream = Simple::new(
            None,
            "Muzak",
            Direction::Playback,
            self.info.name.as_ref().map(|v| v.as_str()),
            "Music",
            &spec,
            None,
            None,
        )?;

        let sample_type = format.sample_type.clone();

        Ok(match sample_type {
            SampleFormat::Float32 => {
                Box::new(PulseStream::<f32>::new(stream, format)) as Box<dyn OutputStream>
            }
            SampleFormat::Signed32 => {
                Box::new(PulseStream::<i32>::new(stream, format)) as Box<dyn OutputStream>
            }
            SampleFormat::Signed24 => {
                Box::new(PulseStream::<I24>::new(stream, format)) as Box<dyn OutputStream>
            }
            SampleFormat::Signed24Packed => {
                Box::new(PulseStream::<I24>::new(stream, format)) as Box<dyn OutputStream>
            }
            SampleFormat::Signed16 => {
                Box::new(PulseStream::<i16>::new(stream, format)) as Box<dyn OutputStream>
            }
            SampleFormat::Unsigned8 => {
                Box::new(PulseStream::<u8>::new(stream, format)) as Box<dyn OutputStream>
            }
            _ => unimplemented!(),
        })
    }

    fn get_supported_formats(&self) -> Result<Vec<SupportedFormat>, InfoError> {
        // pulseaudio doesn't support exclusive mode, only one format is required
        Ok(vec![SupportedFormat {
            originating_provider: "pulse",
            sample_type: get_sample_format(self.info.sample_spec.format),
            sample_rates: self.info.sample_spec.rate..self.info.sample_spec.rate,
            buffer_size: BufferSize::Unknown,
            channels: channel_spec(self.info.channel_map),
        }])
    }

    fn get_default_format(&self) -> Result<FormatInfo, InfoError> {
        let channels = channel_spec(self.info.channel_map);
        Ok(FormatInfo {
            originating_provider: "pulse",
            sample_type: get_sample_format(self.info.sample_spec.format),
            sample_rate: self.info.sample_spec.rate,
            buffer_size: BufferSize::Unknown,
            rate_channel_ratio: channels.count(),
            rate_channel_ratio_fixed: true,
            channels,
        })
    }

    fn get_name(&self) -> Result<String, InfoError> {
        self.info.description.clone().ok_or(InfoError::None)
    }

    fn get_uid(&self) -> Result<String, InfoError> {
        self.info.name.clone().ok_or(InfoError::None)
    }

    fn requires_matching_format(&self) -> bool {
        true
    }
}

trait PulseSample: GetInnerSamples + PartialEq + Copy {}

impl<T> PulseSample for T where T: GetInnerSamples + PartialEq + Copy {}

struct PulseStream<T> {
    phantom: PhantomData<T>,
    stream: Simple,
    format: FormatInfo,
    volume: f64,
}

impl<T> PulseStream<T> {
    pub fn new(stream: Simple, format: FormatInfo) -> Self {
        Self {
            stream,
            format,
            phantom: PhantomData,
            volume: 1.0,
        }
    }
}

impl<T> OutputStream for PulseStream<T>
where
    T: PulseSample,
    [T]: Packed,
    i32: FromWrapper<T>,
    Vec<Vec<T>>: Scale,
{
    fn submit_frame(
        &mut self,
        frame: crate::media::playback::PlaybackFrame,
    ) -> Result<(), crate::devices::errors::SubmissionError> {
        let samples = if self.volume > 0.98 {
            // don't scale if the volume is close to 1, it could lead to (negligable) quality loss
            T::inner(frame.samples)
        } else {
            T::inner(frame.samples).scale(self.volume)
        };

        let interleaved = interleave(samples);
        let packed = if self.format.sample_type == SampleFormat::Signed24 {
            interleaved
                .into_iter()
                .map(|v| i32::from_wrapper(v))
                .collect::<Vec<_>>()
                .as_slice()
                .pack()
        } else {
            interleaved.as_slice().pack()
        };
        let slice = packed.as_slice();

        self.stream.write(slice).map_err(|e| e.into())
    }

    fn close_stream(&mut self) -> Result<(), crate::devices::errors::CloseError> {
        Ok(())
    }

    fn needs_input(&self) -> bool {
        true
    }

    fn get_current_format(&self) -> Result<&FormatInfo, InfoError> {
        Ok(&self.format)
    }

    fn play(&mut self) -> Result<(), crate::devices::errors::StateError> {
        Ok(())
    }

    fn pause(&mut self) -> Result<(), crate::devices::errors::StateError> {
        Ok(())
    }

    fn reset(&mut self) -> Result<(), ResetError> {
        self.stream.flush().map_err(|e| e.into())
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), crate::devices::errors::StateError> {
        self.volume = volume;
        Ok(())
    }
}

trait FromWrapper<T> {
    fn from_wrapper(source: T) -> Self;
}

macro_rules! wrap_u32 {
    ($t: ty) => {
        impl FromWrapper<$t> for i32 {
            fn from_wrapper(source: $t) -> i32 {
                i32::try_from(source).unwrap()
            }
        }
    };
}

wrap_u32!(I24);

// all of this is just to avoid having to write a seperate implemenation of OutputStream for I24
// it's never executed
wrap_u32!(i32);
wrap_u32!(i16);
wrap_u32!(u8);

impl FromWrapper<f32> for i32 {
    fn from_wrapper(source: f32) -> i32 {
        f32::round(source) as i32
    }
}

make_unknown_error_unwrap!(PAErr, ResetError);
make_unknown_error_unwrap!(PAErr, SubmissionError);
make_unknown_error_unwrap!(PAErr, OpenError);
