//! Versioned C ABI surface of the Vut runtime.
//!
//! Runtime entry points are grouped by responsibility into submodules; this
//! root declares the exported symbol names, liveness counters, and re-exports
//! so `vut_runtime::abi::*` remains the single public namespace.
use std::sync::atomic::{AtomicUsize, Ordering};

pub const VERSION: u32 = 13;
pub const ALLOC: &str = "vut_rt_alloc_v1";
pub const REALLOC: &str = "vut_rt_realloc_v1";
pub const FREE: &str = "vut_rt_free_v1";
pub const PANIC: &str = "vut_rt_panic_v1";
pub const BOUNDS_PANIC: &str = "vut_rt_bounds_panic_v1";
pub const BOUNDS_CHECK: &str = "vut_rt_bounds_check_v1";
pub const PRINT: &str = "vut_rt_print_v1";
pub const OUT: &str = "vut_rt_out_v1";
pub const INPUT: &str = "vut_rt_input_v1";
pub const FORMAT_I64: &str = "vut_rt_format_i64_v1";
pub const FORMAT_F64: &str = "vut_rt_format_f64_v1";
pub const F64_MOD: &str = "vut_rt_f64_mod_v1";
pub const CONCAT: &str = "vut_rt_concat_v1";
pub const DROP_STRING: &str = "vut_rt_drop_string_v1";
pub const RETAIN_STRING: &str = "vut_rt_retain_string_v1";
pub const RELEASE_STRING: &str = "vut_rt_release_string_v1";
pub const LIVE_STRING_COUNT: &str = "vut_rt_live_string_count_v1";
pub const LIVE_BYTES_COUNT: &str = "vut_rt_live_bytes_count_v1";
pub const LIVE_LIST_COUNT: &str = "vut_rt_live_list_count_v1";
pub const LIVE_MAP_COUNT: &str = "vut_rt_live_map_count_v1";
pub const LIVE_ALLOCATION_COUNT: &str = "vut_rt_live_allocation_count_v1";
pub const STRING_BYTE_LEN: &str = "vut_rt_string_byte_len_v1";
pub const STRING_CHAR_LEN: &str = "vut_rt_string_char_len_v1";
pub const STRING_IS_EMPTY: &str = "vut_rt_string_is_empty_v1";
pub const STRING_CONTAINS: &str = "vut_rt_string_contains_v1";
pub const STRING_STARTS_WITH: &str = "vut_rt_string_starts_with_v1";
pub const STRING_ENDS_WITH: &str = "vut_rt_string_ends_with_v1";
pub const STRING_TRIM: &str = "vut_rt_string_trim_v1";
pub const STRING_TRIM_START: &str = "vut_rt_string_trim_start_v1";
pub const STRING_TRIM_END: &str = "vut_rt_string_trim_end_v1";
pub const STRING_TO_LOWER: &str = "vut_rt_string_to_lower_v1";
pub const STRING_TO_UPPER: &str = "vut_rt_string_to_upper_v1";
pub const STRING_REPLACE: &str = "vut_rt_string_replace_v1";
pub const STRING_FIND: &str = "vut_rt_string_find_v1";
pub const STRING_SPLIT: &str = "vut_rt_string_split_v1";
pub const STRING_SUBSTRING: &str = "vut_rt_string_substring_v1";
pub const STRING_EQ: &str = "vut_rt_string_eq_v1";
pub const STRING_TO_I64: &str = "vut_rt_string_to_i64_v1";
pub const STRING_TO_F64: &str = "vut_rt_string_to_f64_v1";
pub const STRING_FROM_UTF8: &str = "vut_rt_string_from_utf8_v1";
pub const STRING_TO_BYTES: &str = "vut_rt_string_to_bytes_v1";
pub const STRING_LINES: &str = "vut_rt_string_lines_v1";
pub const STRING_SPLIT_WHITESPACE: &str = "vut_rt_string_split_whitespace_v1";
pub const STRING_CHARS: &str = "vut_rt_string_chars_v1";
pub const STRING_CHAR_AT: &str = "vut_rt_string_char_at_v1";
pub const STRING_REPEAT: &str = "vut_rt_string_repeat_v1";
pub const STRING_PAD_LEFT: &str = "vut_rt_string_pad_left_v1";
pub const STRING_PAD_RIGHT: &str = "vut_rt_string_pad_right_v1";
pub const STRING_STRIP_PREFIX: &str = "vut_rt_string_strip_prefix_v1";
pub const STRING_STRIP_SUFFIX: &str = "vut_rt_string_strip_suffix_v1";
pub const STRING_RFIND: &str = "vut_rt_string_rfind_v1";
pub const STRING_COMPARE: &str = "vut_rt_string_compare_v1";
pub const BYTES_NEW: &str = "vut_rt_bytes_new_v1";
pub const BYTES_CLONE: &str = "vut_rt_bytes_clone_v1";
pub const BYTES_RETAIN: &str = "vut_rt_bytes_retain_v1";
pub const BYTES_RELEASE: &str = "vut_rt_bytes_release_v1";
pub const BYTES_LEN: &str = "vut_rt_bytes_len_v1";
pub const BYTES_IS_EMPTY: &str = "vut_rt_bytes_is_empty_v1";
pub const BYTES_CAPACITY: &str = "vut_rt_bytes_capacity_v1";
pub const BYTES_RESERVE: &str = "vut_rt_bytes_reserve_v1";
pub const BYTES_AT: &str = "vut_rt_bytes_at_v1";
pub const BYTES_SET: &str = "vut_rt_bytes_set_v1";
pub const BYTES_FIRST: &str = "vut_rt_bytes_first_v1";
pub const BYTES_LAST: &str = "vut_rt_bytes_last_v1";
pub const BYTES_SLICE: &str = "vut_rt_bytes_slice_v1";
pub const BYTES_CLEAR: &str = "vut_rt_bytes_clear_v1";
pub const BYTES_TO_LIST: &str = "vut_rt_bytes_to_list_v1";
pub const BYTES_FROM_LIST: &str = "vut_rt_bytes_from_list_v1";
pub const BYTES_TO_STR: &str = "vut_rt_bytes_to_str_v1";
pub const BYTES_UTF8_VALID_UP_TO: &str = "vut_rt_bytes_utf8_valid_up_to_v1";
pub const BYTES_UTF8_ERROR_LEN: &str = "vut_rt_bytes_utf8_error_len_v1";
pub const BYTES_EQ: &str = "vut_rt_bytes_eq_v1";
pub const BYTES_READ_I32: &str = "vut_rt_bytes_read_i32_v1";
pub const BYTES_READ_I64: &str = "vut_rt_bytes_read_i64_v1";
pub const BYTES_BYTE_AT: &str = "vut_rt_bytes_byte_at_v1";
pub const BYTES_PUSH: &str = "vut_rt_bytes_push_v1";
pub const BYTES_EXTEND: &str = "vut_rt_bytes_extend_v1";
pub const BYTES_TRUNCATE: &str = "vut_rt_bytes_truncate_v1";
pub const BYTES_RESIZE: &str = "vut_rt_bytes_resize_v1";
pub const BYTES_FIND: &str = "vut_rt_bytes_find_v1";
pub const BYTES_STARTS_WITH: &str = "vut_rt_bytes_starts_with_v1";
pub const BYTES_ENDS_WITH: &str = "vut_rt_bytes_ends_with_v1";
pub const BYTES_COMPARE: &str = "vut_rt_bytes_compare_v1";
pub const BYTES_TO_HEX: &str = "vut_rt_bytes_to_hex_v1";
pub const BYTES_FROM_HEX: &str = "vut_rt_bytes_from_hex_v1";
pub const BYTES_FROM_HEX_ERROR_INDEX: &str = "vut_rt_bytes_from_hex_error_index_v1";
pub const BYTES_READ_INT: &str = "vut_rt_bytes_read_int_v1";
pub const BYTES_WRITE_INT: &str = "vut_rt_bytes_write_int_v1";
pub const CHAR_FROM_CODE: &str = "vut_rt_char_from_code_v1";
pub const INT_TO_FLOAT: &str = "vut_rt_int_to_float_v1";
pub const INT_ABS: &str = "vut_rt_int_abs_v1";
pub const INT_POW: &str = "vut_rt_int_pow_v1";
pub const INT_MIN: &str = "vut_rt_int_min_v1";
pub const INT_MAX: &str = "vut_rt_int_max_v1";
pub const INT_CLAMP: &str = "vut_rt_int_clamp_v1";
pub const F64_ABS: &str = "vut_rt_f64_abs_v1";
pub const F64_FLOOR: &str = "vut_rt_f64_floor_v1";
pub const F64_CEIL: &str = "vut_rt_f64_ceil_v1";
pub const F64_ROUND: &str = "vut_rt_f64_round_v1";
pub const F64_TRUNC: &str = "vut_rt_f64_trunc_v1";
pub const F64_SQRT: &str = "vut_rt_f64_sqrt_v1";
pub const F64_POW: &str = "vut_rt_f64_pow_v1";
pub const F64_TO_INT: &str = "vut_rt_f64_to_int_v1";
pub const F64_IS_NAN: &str = "vut_rt_f64_is_nan_v1";
pub const F64_IS_FINITE: &str = "vut_rt_f64_is_finite_v1";
pub const F64_MIN: &str = "vut_rt_f64_min_v1";
pub const F64_MAX: &str = "vut_rt_f64_max_v1";
pub const F64_CLAMP: &str = "vut_rt_f64_clamp_v1";
pub const BOOL_TO_STR: &str = "vut_rt_bool_to_str_v1";
pub const LIST_NEW: &str = "vut_rt_list_new_v1";
pub const LIST_RETAIN: &str = "vut_rt_list_retain_v1";
pub const LIST_RELEASE: &str = "vut_rt_list_release_v1";
pub const LIST_LEN: &str = "vut_rt_list_len_v1";
pub const LIST_IS_EMPTY: &str = "vut_rt_list_is_empty_v1";
pub const LIST_CAPACITY: &str = "vut_rt_list_capacity_v1";
pub const LIST_DATA: &str = "vut_rt_list_data_v1";
pub const LIST_RESERVE: &str = "vut_rt_list_reserve_v1";
pub const LIST_PUSH: &str = "vut_rt_list_push_v1";
pub const LIST_AT: &str = "vut_rt_list_at_v1";
pub const LIST_SET: &str = "vut_rt_list_set_v1";
pub const LIST_INSERT: &str = "vut_rt_list_insert_v1";
pub const LIST_REMOVE: &str = "vut_rt_list_remove_v1";
pub const LIST_CLEAR: &str = "vut_rt_list_clear_v1";
pub const LIST_SLICE: &str = "vut_rt_list_slice_v1";
pub const LIST_CONTAINS: &str = "vut_rt_list_contains_v1";
pub const LIST_POP: &str = "vut_rt_list_pop_v1";
pub const LIST_FIRST: &str = "vut_rt_list_first_v1";
pub const LIST_LAST: &str = "vut_rt_list_last_v1";
pub const LIST_FIND_INDEX: &str = "vut_rt_list_find_index_v1";
pub const LIST_EXTEND: &str = "vut_rt_list_extend_v1";
pub const LIST_REVERSE: &str = "vut_rt_list_reverse_v1";
pub const LIST_SWAP: &str = "vut_rt_list_swap_v1";
pub const LIST_SORT: &str = "vut_rt_list_sort_v1";
pub const LIST_TRUNCATE: &str = "vut_rt_list_truncate_v1";
pub const LIST_SHRINK_TO_FIT: &str = "vut_rt_list_shrink_to_fit_v1";
pub const LIST_JOIN: &str = "vut_rt_list_join_v1";
pub const MAP_NEW: &str = "vut_rt_map_new_v1";
pub const MAP_RETAIN: &str = "vut_rt_map_retain_v1";
pub const MAP_RELEASE: &str = "vut_rt_map_release_v1";
pub const MAP_LEN: &str = "vut_rt_map_len_v1";
pub const MAP_IS_EMPTY: &str = "vut_rt_map_is_empty_v1";
pub const MAP_CAPACITY: &str = "vut_rt_map_capacity_v1";
pub const MAP_RESERVE: &str = "vut_rt_map_reserve_v1";
pub const MAP_GET: &str = "vut_rt_map_get_v1";
pub const MAP_SET: &str = "vut_rt_map_set_v1";
pub const MAP_CONTAINS_KEY: &str = "vut_rt_map_contains_key_v1";
pub const MAP_REMOVE: &str = "vut_rt_map_remove_v1";
pub const MAP_CLEAR: &str = "vut_rt_map_clear_v1";
pub const MAP_KEYS: &str = "vut_rt_map_keys_v1";
pub const MAP_VALUES: &str = "vut_rt_map_values_v1";
pub const MAP_GET_OR: &str = "vut_rt_map_get_or_v1";
pub const ARRAY_TO_LIST: &str = "vut_rt_array_to_list_v1";
pub const ARRAY_REVERSE: &str = "vut_rt_array_reverse_v1";
pub const ARRAY_SORT: &str = "vut_rt_array_sort_v1";
pub const ARRAY_CONTAINS: &str = "vut_rt_array_contains_v1";

pub const VUTCON_SPAWN: &str = "vut_rt_vutcon_spawn_v1";
pub const VUTCON_LIVE_COUNT: &str = "vut_rt_vutcon_live_count_v1";
pub const TASK_NEW: &str = "vut_rt_task_new_v1";
pub const TASK_POLL: &str = "vut_rt_task_poll_v1";
pub const TASK_RESULT: &str = "vut_rt_task_result_v1";
pub const EXECUTOR_RUN: &str = "vut_rt_executor_run_v1";
pub const EXECUTOR_DRAIN: &str = "vut_rt_executor_drain_v1";
pub const RESOURCE_NEW: &str = "vut_rt_resource_new_v1";
pub const RESOURCE_PTR: &str = "vut_rt_resource_ptr_v1";
pub const RESOURCE_RELEASE: &str = "vut_rt_resource_release_v1";
pub const RESOURCE_LIVE_COUNT: &str = "vut_rt_resource_live_count_v1";
pub const ASYNC_NEW: &str = "vut_rt_async_new_v1";
pub const ASYNC_DROP: &str = "vut_rt_async_drop_v1";
pub const ASYNC_AWAIT: &str = "vut_rt_future_await_v1";
pub const ASYNC_AWAIT_CHILD: &str = "vut_rt_async_await_child_v1";
pub const ASYNC_ON_RESUME: &str = "vut_rt_async_on_resume_v1";
pub const FRAME_ALLOC: &str = "vut_rt_frame_alloc_v1";
pub const FRAME_FREE: &str = "vut_rt_frame_free_v1";
pub const FRAME_LIVE_COUNT: &str = "vut_rt_frame_live_count_v1";
pub const INTERFACE_NEW: &str = "vut_rt_interface_new";
pub const INTERFACE_DATA: &str = "vut_rt_interface_data";
pub const INTERFACE_SET_VTABLE: &str = "vut_rt_interface_set_vtable";
pub const INTERFACE_VTABLE: &str = "vut_rt_interface_vtable";
pub const INTERFACE_RETAIN: &str = "vut_rt_interface_retain";
pub const INTERFACE_RELEASE: &str = "vut_rt_interface_release";

pub use crate::bytes::{
    vut_rt_bytes_at_v1, vut_rt_bytes_byte_at_v1, vut_rt_bytes_capacity_v1, vut_rt_bytes_clear_v1,
    vut_rt_bytes_clone_v1, vut_rt_bytes_compare_v1, vut_rt_bytes_ends_with_v1, vut_rt_bytes_eq_v1,
    vut_rt_bytes_extend_v1, vut_rt_bytes_find_v1, vut_rt_bytes_first_v1,
    vut_rt_bytes_from_hex_error_index_v1, vut_rt_bytes_from_hex_v1, vut_rt_bytes_is_empty_v1,
    vut_rt_bytes_last_v1, vut_rt_bytes_len_v1, vut_rt_bytes_new_v1, vut_rt_bytes_push_v1,
    vut_rt_bytes_read_int_v1, vut_rt_bytes_release_v1, vut_rt_bytes_reserve_v1,
    vut_rt_bytes_resize_v1, vut_rt_bytes_retain_v1, vut_rt_bytes_set_v1, vut_rt_bytes_slice_v1,
    vut_rt_bytes_starts_with_v1, vut_rt_bytes_to_hex_v1, vut_rt_bytes_truncate_v1,
    vut_rt_bytes_utf8_error_len_v1, vut_rt_bytes_utf8_valid_up_to_v1, vut_rt_bytes_write_int_v1,
    vut_rt_live_bytes_count_v1,
};

mod interface;
mod io;
mod join;
mod list;
mod map;
mod numeric;
mod ownership;
mod panic;
mod string;

pub use crate::async_handle::{
    vut_rt_async_drop_v1, vut_rt_async_live_count_v1, vut_rt_async_new_v1,
    vut_rt_async_on_resume_v1, vut_rt_async_poll_v1, vut_rt_async_register_v1,
    vut_rt_async_signal_v1,
};
pub use crate::awaiting::{vut_rt_async_await_child_v1, vut_rt_future_await_v1};
pub use crate::executor::{vut_rt_executor_drain_v1, vut_rt_executor_run_v1};
pub use crate::frame::{vut_rt_frame_alloc_v1, vut_rt_frame_free_v1, vut_rt_frame_live_count_v1};
pub use crate::resource::{
    vut_rt_resource_live_count_v1, vut_rt_resource_new_v1, vut_rt_resource_ptr_v1,
    vut_rt_resource_release_v1,
};
pub use crate::task::{vut_rt_task_new_v1, vut_rt_task_poll_v1, vut_rt_task_result_v1};
pub use crate::vutcon::{vut_rt_vutcon_live_count_v1, vut_rt_vutcon_spawn_v1};
pub use io::{vut_rt_input_v1, vut_rt_out_v1, vut_rt_print_v1};
pub use join::vut_rt_list_join_v1;
pub use list::{
    ManagedList, vut_rt_array_contains_v1, vut_rt_array_reverse_v1, vut_rt_array_sort_v1,
    vut_rt_array_to_list_v1, vut_rt_bytes_from_list_v1, vut_rt_bytes_to_list_v1, vut_rt_list_at_v1,
    vut_rt_list_capacity_v1, vut_rt_list_clear_v1, vut_rt_list_contains_v1, vut_rt_list_data_v1,
    vut_rt_list_extend_v1, vut_rt_list_find_index_v1, vut_rt_list_first_v1, vut_rt_list_insert_v1,
    vut_rt_list_is_empty_v1, vut_rt_list_last_v1, vut_rt_list_len_v1, vut_rt_list_new_v1,
    vut_rt_list_pop_v1, vut_rt_list_push_v1, vut_rt_list_release_v1, vut_rt_list_remove_v1,
    vut_rt_list_reserve_v1, vut_rt_list_retain_v1, vut_rt_list_reverse_v1, vut_rt_list_set_v1,
    vut_rt_list_shrink_to_fit_v1, vut_rt_list_slice_v1, vut_rt_list_sort_v1, vut_rt_list_swap_v1,
    vut_rt_list_truncate_v1,
};
pub use map::{
    ManagedMap, vut_rt_map_capacity_v1, vut_rt_map_clear_v1, vut_rt_map_contains_key_v1,
    vut_rt_map_get_or_v1, vut_rt_map_get_v1, vut_rt_map_is_empty_v1, vut_rt_map_keys_v1,
    vut_rt_map_len_v1, vut_rt_map_new_v1, vut_rt_map_release_v1, vut_rt_map_remove_v1,
    vut_rt_map_reserve_v1, vut_rt_map_retain_v1, vut_rt_map_set_v1, vut_rt_map_values_v1,
};
pub use numeric::{
    vut_rt_bool_to_str_v1, vut_rt_f64_abs_v1, vut_rt_f64_ceil_v1, vut_rt_f64_clamp_v1,
    vut_rt_f64_floor_v1, vut_rt_f64_is_finite_v1, vut_rt_f64_is_nan_v1, vut_rt_f64_max_v1,
    vut_rt_f64_min_v1, vut_rt_f64_pow_v1, vut_rt_f64_round_v1, vut_rt_f64_sqrt_v1,
    vut_rt_f64_to_int_v1, vut_rt_f64_trunc_v1, vut_rt_int_abs_v1, vut_rt_int_clamp_v1,
    vut_rt_int_max_v1, vut_rt_int_min_v1, vut_rt_int_pow_v1,
};
pub use ownership::{
    vut_rt_slot_release_bytes_v1, vut_rt_slot_release_list_v1, vut_rt_slot_release_map_v1,
    vut_rt_slot_release_string_v1, vut_rt_slot_retain_bytes_v1, vut_rt_slot_retain_list_v1,
    vut_rt_slot_retain_map_v1, vut_rt_slot_retain_string_v1,
};
pub use panic::{
    bounds_op, signed_index, vut_rt_bounds_check_v1, vut_rt_bounds_panic_v1, vut_rt_panic_v1,
};
pub use string::{
    ManagedString, managed_string, string_value, vut_rt_bytes_to_str_v1, vut_rt_char_from_code_v1,
    vut_rt_concat_v1, vut_rt_drop_string_v1, vut_rt_format_f64_v1, vut_rt_format_i64_v1,
    vut_rt_int_to_float_v1, vut_rt_release_string_v1, vut_rt_retain_string_v1,
    vut_rt_string_byte_len_v1, vut_rt_string_char_at_v1, vut_rt_string_char_len_v1,
    vut_rt_string_chars_v1, vut_rt_string_compare_v1, vut_rt_string_contains_v1,
    vut_rt_string_ends_with_v1, vut_rt_string_find_v1, vut_rt_string_from_utf8_v1,
    vut_rt_string_is_empty_v1, vut_rt_string_lines_v1, vut_rt_string_pad_left_v1,
    vut_rt_string_pad_right_v1, vut_rt_string_repeat_v1, vut_rt_string_replace_v1,
    vut_rt_string_rfind_v1, vut_rt_string_split_v1, vut_rt_string_split_whitespace_v1,
    vut_rt_string_starts_with_v1, vut_rt_string_strip_prefix_v1, vut_rt_string_strip_suffix_v1,
    vut_rt_string_substring_v1, vut_rt_string_to_bytes_v1, vut_rt_string_to_f64_v1,
    vut_rt_string_to_i64_v1, vut_rt_string_to_lower_v1, vut_rt_string_to_upper_v1,
    vut_rt_string_trim_end_v1, vut_rt_string_trim_start_v1, vut_rt_string_trim_v1,
};
pub(crate) static LIVE_STRINGS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static LIVE_LISTS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static LIVE_MAPS: AtomicUsize = AtomicUsize::new(0);

#[must_use]
pub fn live_string_allocations() -> usize {
    LIVE_STRINGS.load(Ordering::Relaxed)
}

#[must_use]
pub fn live_list_allocations() -> usize {
    LIVE_LISTS.load(Ordering::Relaxed)
}

#[must_use]
pub fn live_map_allocations() -> usize {
    LIVE_MAPS.load(Ordering::Relaxed)
}

/// Total number of live managed allocations of every kind.
#[must_use]
pub fn live_managed_allocations() -> usize {
    live_string_allocations()
        + crate::bytes::live_bytes_allocations()
        + live_list_allocations()
        + live_map_allocations()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeAbi {
    pub version: u32,
}
pub const CURRENT: RuntimeAbi = RuntimeAbi { version: VERSION };

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_live_string_count_v1() -> usize {
    live_string_allocations()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_live_list_count_v1() -> usize {
    live_list_allocations()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_live_map_count_v1() -> usize {
    live_map_allocations()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_live_allocation_count_v1() -> usize {
    live_managed_allocations()
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_f64_mod_v1(left: f64, right: f64) -> f64 {
    left % right
}
