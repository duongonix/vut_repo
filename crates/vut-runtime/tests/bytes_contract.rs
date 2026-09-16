use vut_runtime::{abi, bytes};

/// Builds a `bytes` buffer from raw byte values through the list bridge.
unsafe fn bytes_of(values: &[u8]) -> *mut bytes::ManagedBytes {
    let list =
        unsafe { abi::vut_rt_list_new_v1(1, 1, std::ptr::null(), std::ptr::null(), values.len()) };
    for value in values {
        unsafe { abi::vut_rt_list_push_v1(list, std::ptr::from_ref(value)) };
    }
    let blob = unsafe { abi::vut_rt_bytes_from_list_v1(list) };
    unsafe { abi::vut_rt_list_release_v1(list) };
    blob
}

#[test]
fn bytes_access_is_bounds_safe_with_zero_fallback() {
    // SAFETY: every handle is created and released within this test, and `out`
    // always points to initialized storage.
    unsafe {
        let blob = bytes_of(&[0x58, 0xff, 0x41]);
        assert_eq!(abi::vut_rt_bytes_len_v1(blob), 3);
        assert_eq!(abi::vut_rt_bytes_is_empty_v1(blob), 0);

        let mut out = 0_u8;
        assert_eq!(
            abi::vut_rt_bytes_at_v1(blob, 0, std::ptr::from_mut(&mut out)),
            1
        );
        assert_eq!(out, 0x58);
        assert_eq!(abi::vut_rt_bytes_set_v1(blob, 0, 0x5a), 1);
        assert_eq!(
            abi::vut_rt_bytes_at_v1(blob, 0, std::ptr::from_mut(&mut out)),
            1
        );
        assert_eq!(out, 0x5a);

        out = 7;
        assert_eq!(
            abi::vut_rt_bytes_at_v1(blob, 99, std::ptr::from_mut(&mut out)),
            0
        );
        assert_eq!(out, 0, "out must be zeroed on out-of-bounds access");
        assert_eq!(abi::vut_rt_bytes_set_v1(blob, 99, 1), 0);

        let empty = abi::vut_rt_bytes_new_v1();
        out = 7;
        assert_eq!(
            abi::vut_rt_bytes_first_v1(empty, std::ptr::from_mut(&mut out)),
            0
        );
        assert_eq!(out, 0);
        out = 7;
        assert_eq!(
            abi::vut_rt_bytes_last_v1(empty, std::ptr::from_mut(&mut out)),
            0
        );
        assert_eq!(out, 0);

        assert!(abi::vut_rt_bytes_slice_v1(blob, 5, 9).is_null());
        let sliced = abi::vut_rt_bytes_slice_v1(blob, 1, 3);
        assert_eq!(abi::vut_rt_bytes_len_v1(sliced), 2);
        abi::vut_rt_bytes_release_v1(sliced);
        abi::vut_rt_bytes_release_v1(empty);
        abi::vut_rt_bytes_release_v1(blob);
    }
}

#[test]
fn bytes_utf8_classification_reports_exact_locations() {
    // SAFETY: handles are created and released within the test.
    unsafe {
        let valid = bytes_of("Xin chào".as_bytes());
        assert_eq!(
            abi::vut_rt_bytes_utf8_valid_up_to_v1(valid),
            "Xin chào".len()
        );
        assert_eq!(abi::vut_rt_bytes_utf8_error_len_v1(valid), 0);
        assert!(!abi::vut_rt_bytes_to_str_v1(valid).is_null());
        abi::vut_rt_bytes_release_v1(valid);

        let invalid = bytes_of(&[0x41, 0x80]);
        assert_eq!(abi::vut_rt_bytes_utf8_valid_up_to_v1(invalid), 1);
        assert_eq!(abi::vut_rt_bytes_utf8_error_len_v1(invalid), 1);
        assert!(abi::vut_rt_bytes_to_str_v1(invalid).is_null());
        abi::vut_rt_bytes_release_v1(invalid);

        let truncated = bytes_of(&[0xe2, 0x82]);
        assert_eq!(abi::vut_rt_bytes_utf8_valid_up_to_v1(truncated), 0);
        assert_eq!(abi::vut_rt_bytes_utf8_error_len_v1(truncated), 0);
        abi::vut_rt_bytes_release_v1(truncated);
    }
}

#[test]
fn bytes_equality_compares_contents_not_identity() {
    // SAFETY: handles are created and released within the test.
    unsafe {
        let left = bytes_of(&[1, 2, 3]);
        let same = abi::vut_rt_bytes_clone_v1(left);
        let different = bytes_of(&[1, 2, 4]);
        assert_eq!(abi::vut_rt_bytes_eq_v1(left, same), 1);
        assert_eq!(abi::vut_rt_bytes_eq_v1(left, different), 0);
        abi::vut_rt_bytes_release_v1(different);
        abi::vut_rt_bytes_release_v1(same);
        abi::vut_rt_bytes_release_v1(left);
    }
}

#[test]
fn bytes_release_accounting_returns_to_baseline() {
    let before = bytes::live_bytes_allocations();
    // SAFETY: handles are created and released within the test.
    unsafe {
        let blob = abi::vut_rt_bytes_new_v1();
        assert_eq!(bytes::live_bytes_allocations(), before + 1);
        let cloned = abi::vut_rt_bytes_clone_v1(blob);
        assert_eq!(bytes::live_bytes_allocations(), before + 2);
        abi::vut_rt_bytes_retain_v1(cloned);
        abi::vut_rt_bytes_release_v1(cloned);
        assert_eq!(bytes::live_bytes_allocations(), before + 2);
        abi::vut_rt_bytes_release_v1(cloned);
        abi::vut_rt_bytes_release_v1(blob);
    }
    assert_eq!(bytes::live_bytes_allocations(), before);
}

#[test]
fn bytes_large_buffer_grows_without_corruption() {
    // SAFETY: handles are created and released within the test.
    unsafe {
        let values: Vec<u8> = (0..1000)
            .map(|index| u8::try_from(index % 256).unwrap())
            .collect();
        let blob = bytes_of(&values);
        assert_eq!(abi::vut_rt_bytes_len_v1(blob), 1000);
        abi::vut_rt_bytes_reserve_v1(blob, 8192);
        assert!(abi::vut_rt_bytes_capacity_v1(blob) >= 8192);
        let mut out = 0_u8;
        abi::vut_rt_bytes_at_v1(blob, 999, std::ptr::from_mut(&mut out));
        assert_eq!(out, values[999]);
        abi::vut_rt_bytes_release_v1(blob);
    }
}
