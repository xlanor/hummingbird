use bitflags::Flags;
use libpulse_binding::{
    channelmap::{Map, Position},
    sample::{Format, Spec},
    stream::Direction,
};
use libpulse_simple_binding::Simple;
use pulsectl::controllers::{types::DeviceInfo, DeviceControl, SinkController};

use crate::devices::{
    errors::{FindError, InfoError, InitializationError, ListError, OpenError},
    format::{BufferSize, ChannelSpec, Channels, FormatInfo, SampleFormat, SupportedFormat},
    traits::{Device, DeviceProvider, OutputStream},
};

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
            .map_err(|_| ListError::Unknown)?
            .into_iter()
            .map(|d| Box::new(PulseDevice::from(d)) as Box<dyn Device>)
            .collect())
    }

    fn get_default_device(&mut self) -> Result<Box<dyn Device>, FindError> {
        Ok(Box::new(PulseDevice::from(
            self.controller
                .get_default_device()
                .map_err(|_| FindError::Unknown)?,
        )) as Box<dyn Device>)
    }

    fn get_device_by_uid(&mut self, id: &String) -> Result<Box<dyn Device>, FindError> {
        Ok(Box::new(PulseDevice::from(
            self.controller
                .get_device_by_name(id)
                .map_err(|_| FindError::Unknown)?,
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
        let spec = pulse_spec(format);
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
        )
        .map_err(|_| OpenError::Unknown)?;

        todo!()
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
        Ok(FormatInfo {
            originating_provider: "pulse",
            sample_type: get_sample_format(self.info.sample_spec.format),
            sample_rate: self.info.sample_spec.rate,
            buffer_size: BufferSize::Unknown,
            channels: channel_spec(self.info.channel_map),
        })
    }

    fn get_name(&self) -> Result<String, InfoError> {
        self.info.description.clone().ok_or(InfoError::Unknown)
    }

    fn get_uid(&self) -> Result<String, InfoError> {
        self.info.name.clone().ok_or(InfoError::Unknown)
    }

    fn requires_matching_format(&self) -> bool {
        true
    }
}

struct PulseStream {
    stream: Simple,
}

impl From<Simple> for PulseStream {
    fn from(stream: Simple) -> Self {
        Self { stream }
    }
}

impl OutputStream for PulseStream {
    fn submit_frame(
        &mut self,
        frame: crate::media::playback::PlaybackFrame,
    ) -> Result<(), crate::devices::errors::SubmissionError> {
        todo!()
    }

    fn close_stream(&mut self) -> Result<(), crate::devices::errors::CloseError> {
        Ok(())
    }

    fn needs_input(&self) -> bool {
        todo!()
    }

    fn get_current_format(&self) -> Result<&FormatInfo, InfoError> {
        todo!()
    }

    fn play(&mut self) -> Result<(), crate::devices::errors::StateError> {
        todo!()
    }

    fn pause(&mut self) -> Result<(), crate::devices::errors::StateError> {
        todo!()
    }

    fn reset(&mut self) -> Result<(), crate::devices::errors::ResetError> {
        todo!()
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), crate::devices::errors::StateError> {
        todo!()
    }
}
