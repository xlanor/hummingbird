#![allow(dead_code)]

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum OpenError {
    FileCorrupt,
    UnsupportedFormat,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum CloseError {
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PlaybackStartError {
    NothingOpen,
    NothingToPlay,
    Undecodable,
    BrokenContainer,
    ContainerSupportedButNotCodec,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PlaybackStopError {
    NothingOpen,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PlaybackReadError {
    NothingOpen,
    NeverStarted,
    Eof,
    Unknown,
    DecodeFatal,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum MetadataError {
    NothingOpen,
    OperationUnsupported,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum FrameDurationError {
    NothingOpen,
    NeverDecoded,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TrackDurationError {
    NothingOpen,
    NeverStarted,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SeekError {
    NothingOpen,
    Unknown,
}
