use crate::cstring;
use std::ptr;
use crate::sys;

// Safe wrapper around AVDictionary
pub struct AVDict(*mut sys::AVDictionary);

impl AVDict {
    pub fn new() -> Self {
        Self(ptr::null_mut())
    }

    pub fn as_mut_ptr(&mut self) -> *mut *mut sys::AVDictionary {
        &mut self.0
    }

    pub fn inner(&self) -> *mut sys::AVDictionary {
        self.0
    }

    pub fn set(&mut self, key: &str, value: &str) {
        let key = cstring!(key);
        let value = cstring!(value);
        unsafe {
            sys::av_dict_set(&mut self.0, key.as_ptr(), value.as_ptr(), 0);
        }
    }
}

impl Drop for AVDict {
    fn drop(&mut self) {
        unsafe {
            sys::av_dict_free(&mut self.0);
        }
    }
}
