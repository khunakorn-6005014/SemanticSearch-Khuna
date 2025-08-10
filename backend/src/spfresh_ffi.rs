// backend/src/spfresh_ffi.rs

use libc::{c_char, c_float, c_int, int64_t};

#[repr(C)]
pub struct SPFHandleOpaque;
pub type SPFHandle = *mut SPFHandleOpaque;

extern "C" {
    pub fn spf_init_index(path: *const c_char, out_handle: *mut SPFHandle) -> c_int;
    pub fn spf_add_vector(handle: SPFHandle, vec: *const c_float, dim: c_int, id: int64_t) -> c_int;
    pub fn spf_search(
        handle: SPFHandle,
        query: *const c_float,
        dim: c_int,
        k: c_int,
        out_ids: *mut int64_t,
        out_scores: *mut c_float,
    ) -> c_int;
    pub fn spf_close(handle: SPFHandle) -> c_int;
}