use crate::windows_recorder::sys;

#[derive(Copy, Clone)]
pub struct CodecParPtr(pub *mut sys::AVCodecParameters);
unsafe impl Send for CodecParPtr {}
unsafe impl Sync for CodecParPtr {}
