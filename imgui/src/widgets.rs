use imgui_sys;
use std::marker::PhantomData;
use std::ptr;

use super::{Ui, ImStr};

#[must_use]
pub struct CollapsingHeader<'ui, 'p> {
    label: ImStr<'p>,
    str_id: Option<ImStr<'p>>,
    display_frame: bool,
    default_open: bool,
    _phantom: PhantomData<&'ui Ui<'ui>>
}

impl<'ui, 'p> CollapsingHeader<'ui, 'p> {
    pub fn new(label: ImStr<'p>) -> Self {
        CollapsingHeader {
            label: label,
            str_id: None,
            display_frame: true,
            default_open: false,
            _phantom: PhantomData
        }
    }
    #[inline]
    pub fn str_id(self, str_id: ImStr<'p>) -> Self {
        CollapsingHeader {
            str_id: Some(str_id),
            .. self
        }
    }
    #[inline]
    pub fn display_frame(self, display_frame: bool) -> Self {
        CollapsingHeader {
            display_frame: display_frame,
            .. self
        }
    }
    #[inline]
    pub fn default_open(self, default_open: bool) -> Self {
        CollapsingHeader {
            default_open: default_open,
            .. self
        }
    }
    pub fn build(self) -> bool {
        unsafe {
            imgui_sys::igCollapsingHeader(
                self.label.as_ptr(),
                self.str_id.map(|x| x.as_ptr()).unwrap_or(ptr::null()),
                self.display_frame,
                self.default_open
                )
        }
    }
}
