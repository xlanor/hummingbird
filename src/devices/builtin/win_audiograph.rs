use std::{ptr::null, slice::from_raw_parts_mut};

use rb::{Producer, RbConsumer, RbProducer, SpscRb, RB};
use tracing::{error, info};
use windows::{
    core::{IUnknown, Interface, HSTRING},
    Devices::Enumeration::{DeviceClass, DeviceInformation},
    Foundation::TypedEventHandler,
    Media::{
        Audio::{
            AudioDeviceOutputNode, AudioFrameInputNode, AudioGraph, AudioGraphSettings,
            FrameInputNodeQuantumStartedEventArgs,
        },
        AudioBufferAccessMode, AudioFrame,
        Render::AudioRenderCategory,
    },
    Win32::System::WinRT::IMemoryBufferByteAccess,
};

use crate::{
    devices::{
        errors::{
            CloseError, FindError, InfoError, InitializationError, ListError, OpenError,
            ResetError, StateError, SubmissionError,
        },
        format::{BufferSize, ChannelSpec, FormatInfo, SampleFormat, SupportedFormat},
        traits::{Device, DeviceProvider, OutputStream},
        util::{interleave, Packed},
    },
    media::playback::{GetInnerSamples, Mute, PlaybackFrame},
};

/// Windows Audio Graph backend
///
/// Audio Graph is the most managed of the Windows backends: you can throw nearly any stream at
/// any device and have it play. Unlike WASAPI, it supports multiple output formats to the same
/// device, and unlike XAudio2 and DirectSound, it supports low-latency mode.
pub struct AudioGraphProvider {}

impl Default for AudioGraphProvider {
    fn default() -> Self {
        Self {}
    }
}

impl DeviceProvider for AudioGraphProvider {
    fn initialize(&mut self) -> Result<(), InitializationError> {
        Ok(())
    }

    fn get_devices(&mut self) -> Result<Vec<Box<dyn Device>>, ListError> {
        let devices = DeviceInformation::FindAllAsyncDeviceClass(DeviceClass::AudioRender);

        Ok(devices
            .and_then(|v| v.get())
            .map_err(|_| ListError::Unknown)?
            .into_iter()
            .map(|device| Box::new(AudioGraphDevice::from(device)) as Box<dyn Device>)
            .collect())
    }

    fn get_default_device(&mut self) -> Result<Box<dyn Device>, FindError> {
        Ok(Box::new(AudioGraphDevice::new()) as Box<dyn Device>)
    }

    fn get_device_by_uid(&mut self, id: &str) -> Result<Box<dyn Device>, FindError> {
        let devices_result = DeviceInformation::FindAllAsyncDeviceClass(DeviceClass::AudioRender);

        let Ok(devices) = devices_result.and_then(|v| v.get()) else {
            return Err(FindError::Unknown);
        };

        devices
            .into_iter()
            .find(|v| v.Id().unwrap_or_default() == id)
            .ok_or(FindError::DeviceDoesNotExist)
            .map(|device| Box::new(AudioGraphDevice::from(device)) as Box<dyn Device>)
    }
}

pub struct AudioGraphDevice {
    graph: AudioGraph,
    device_out: AudioDeviceOutputNode,
}

impl AudioGraphDevice {
    pub fn new() -> Self {
        let settings = AudioGraphSettings::Create(AudioRenderCategory::Media)
            .expect("Could not create default audio settings!");

        let graph_async =
            AudioGraph::CreateAsync(&settings).expect("Could not create default audio graph!");

        let graph = graph_async
            .get()
            .expect("Waiting for asynchronous operation failed: AudioGraph::CreateAsync");

        if let Err(status) = graph.Status() {
            error!("Error initializing graph! {:?}", status)
        }

        let graph_final = graph.Graph().unwrap();

        let device_out = graph_final
            .CreateDeviceOutputNodeAsync()
            .expect("Could not attach audio device to audio graph")
            .get()
            .expect("couldn't get attached audio device");

        if let Err(status) = device_out.Status() {
            error!("Error initializing output device! {:?}", status)
        }

        AudioGraphDevice {
            graph: graph_final,
            device_out: device_out.DeviceOutputNode().unwrap(),
        }
    }
}

impl From<DeviceInformation> for AudioGraphDevice {
    fn from(value: DeviceInformation) -> Self {
        let settings = AudioGraphSettings::Create(AudioRenderCategory::Media)
            .expect("Could not create default audio settings!");

        settings
            .SetPrimaryRenderDevice(&value)
            .expect("Could not set audio device!");

        let graph_async =
            AudioGraph::CreateAsync(&settings).expect("Could not create default audio graph!");

        let graph = graph_async
            .get()
            .expect("Waiting for asynchronous operation failed: AudioGraph::CreateAsync");

        if let Err(status) = graph.Status() {
            error!("Error initializing graph! {:?}", status)
        }

        let graph_final = graph.Graph().unwrap();

        let device_out = graph_final
            .CreateDeviceOutputNodeAsync()
            .expect("Could not attach audio device to audio graph")
            .get()
            .expect("couldn't get attached audio device");

        if let Err(status) = device_out.Status() {
            error!("Error initializing output device! {:?}", status)
        }

        AudioGraphDevice {
            graph: graph_final,
            device_out: device_out.DeviceOutputNode().unwrap(),
        }
    }
}

impl Device for AudioGraphDevice {
    fn open_device(&mut self, format: FormatInfo) -> Result<Box<dyn OutputStream>, OpenError> {
        self.graph.Start().map_err(|_| OpenError::Unknown)?;
        self.device_out.Start().map_err(|_| OpenError::Unknown)?;

        let properties = self
            .graph
            .EncodingProperties()
            .map_err(|_| OpenError::Unknown)?;

        properties
            .SetChannelCount(format.channels.count() as u32)
            .map_err(|_| OpenError::InvalidSampleFormat)?;

        let input_node = self
            .graph
            .CreateFrameInputNodeWithFormat(&properties)
            .map_err(|_| OpenError::Unknown)?;

        input_node
            .AddOutgoingConnection(&self.device_out)
            .map_err(|_| OpenError::Unknown)?;

        input_node.Stop().map_err(|_| OpenError::Unknown)?;

        let buffer_size = match format.buffer_size {
            BufferSize::Fixed(v) => v,
            _ => panic!("invalid buffer_size (wrong provider?)"),
        };

        let rb: SpscRb<u8> = SpscRb::new(buffer_size as usize * 4);
        let cons = rb.consumer();
        let prod = rb.producer();

        let handler =
            TypedEventHandler::<AudioFrameInputNode, FrameInputNodeQuantumStartedEventArgs>::new(
                move |sender, args| {
                    let samples = args.as_ref().unwrap().RequiredSamples()?;

                    if samples == 0 {
                        return windows_result::Result::Ok(());
                    }

                    let required_capacity = (samples as u32) * size_of::<f32>() as u32;

                    let frame = AudioFrame::Create(required_capacity)?;
                    let lock = frame.LockBuffer(AudioBufferAccessMode::Write)?;
                    let reference = lock.CreateReference()?;

                    // why?
                    let write = reference.cast::<IMemoryBufferByteAccess>()?;

                    let slice;

                    unsafe {
                        // what the fuck?
                        let mut value = std::ptr::null_mut();
                        let mut capacity = 0;
                        write
                            .GetBuffer(&mut value, &mut capacity)
                            .expect("this must work or memory will be corrupted");

                        slice = from_raw_parts_mut(value, capacity as usize);
                    }

                    let read = cons.read(slice).unwrap_or(0);
                    // should be fine? IEEE says that 0.0 is 0x00000000...
                    slice[read..].iter_mut().for_each(|v| *v = 0);

                    //info!("read {:?} bytes (out of {:?})", read, required_capacity);

                    //info!("adding");
                    sender.as_ref().unwrap().AddFrame(&frame)?;

                    //info!("done");
                    windows_result::Result::Ok(())
                },
            );

        input_node
            .QuantumStarted(&handler)
            .map_err(|_| OpenError::Unknown)?;

        let stream = AudioGraphStream {
            node: input_node,
            producer: prod,
            format,
        };

        Ok(Box::new(stream) as Box<dyn OutputStream>)
    }

    fn get_supported_formats(&self) -> Result<Vec<SupportedFormat>, InfoError> {
        let properties = self
            .graph
            .EncodingProperties()
            .map_err(|_| InfoError::Unknown)?;
        let sample_rate = properties.SampleRate().map_err(|_| InfoError::Unknown)?;
        let buffer_size = self
            .graph
            .SamplesPerQuantum()
            .map_err(|_| InfoError::Unknown)?;
        let channels = properties.ChannelCount().map_err(|_| InfoError::Unknown)?;

        Ok(vec![SupportedFormat {
            originating_provider: "win_audiograph",
            sample_type: SampleFormat::Float32,
            sample_rates: sample_rate..sample_rate,
            buffer_size: BufferSize::Fixed(buffer_size as u32),
            channels: ChannelSpec::Count(channels as u16),
        }])
    }

    fn get_default_format(&self) -> Result<FormatInfo, InfoError> {
        let properties = self
            .graph
            .EncodingProperties()
            .map_err(|_| InfoError::Unknown)?;
        let sample_rate = properties.SampleRate().map_err(|_| InfoError::Unknown)?;
        let buffer_size = self
            .graph
            .SamplesPerQuantum()
            .map_err(|_| InfoError::Unknown)?;
        let channels = properties.ChannelCount().map_err(|_| InfoError::Unknown)?;

        Ok(FormatInfo {
            originating_provider: "win_audiograph",
            sample_type: SampleFormat::Float32,
            sample_rate,
            buffer_size: BufferSize::Fixed(buffer_size as u32),
            channels: ChannelSpec::Count(channels as u16),
            rate_channel_ratio: 2,
            rate_channel_ratio_fixed: true,
        })
    }

    fn get_name(&self) -> Result<String, InfoError> {
        let device = self
            .graph
            .PrimaryRenderDevice()
            .map_err(|_| InfoError::Unknown)?;

        device
            .Name()
            .map_err(|_| InfoError::Unknown)
            .map(|v| v.to_string())
    }

    fn get_uid(&self) -> Result<String, InfoError> {
        let device = self
            .graph
            .PrimaryRenderDevice()
            .map_err(|_| InfoError::Unknown)?;

        device
            .Id()
            .map_err(|_| InfoError::Unknown)
            .map(|v| v.to_string())
    }

    fn requires_matching_format(&self) -> bool {
        true
    }
}

pub struct AudioGraphStream {
    pub node: AudioFrameInputNode,
    pub producer: Producer<u8>,
    pub format: FormatInfo,
}

impl OutputStream for AudioGraphStream {
    fn submit_frame(&mut self, frame: PlaybackFrame) -> Result<(), SubmissionError> {
        self.node.Start().expect("couldn't start");

        info!("submitting samples");
        let samples = f32::inner(frame.samples);
        let packed = interleave(samples).pack();
        let mut slice: &[u8] = &packed;

        while let Some(written) = self.producer.write_blocking(slice) {
            slice = &slice[written..];
        }

        Ok(())
    }

    fn close_stream(&mut self) -> Result<(), CloseError> {
        self.node.Close().map_err(|_| CloseError::Unknown)
    }

    fn needs_input(&self) -> bool {
        true
    }

    fn get_current_format(&self) -> Result<&FormatInfo, InfoError> {
        Ok(&self.format)
    }

    fn play(&mut self) -> Result<(), StateError> {
        info!("Playing");
        self.node.Start().map_err(|_| StateError::Unknown)
    }

    fn pause(&mut self) -> Result<(), StateError> {
        self.node.Stop().map_err(|_| StateError::Unknown)
    }

    fn reset(&mut self) -> Result<(), ResetError> {
        self.node.Reset().map_err(|_| ResetError::Unknown)
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), StateError> {
        self.node
            .SetOutgoingGain(volume)
            .map_err(|_| StateError::Unknown)
    }
}
