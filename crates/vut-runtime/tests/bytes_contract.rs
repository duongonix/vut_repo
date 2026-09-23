use vut_runtime::{abi, bytes};

// Allocation accounting is process-global; serialize these buffer-owning tests.
static BUFFER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
fn bytes_access_and_empty_fallback_contracts() {
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
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
fn bytes_explicit_oob_traps() {
    const CHILD: &str = "VUT_BYTES_CONTRACT_OOB";
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
    if let Ok(operation) = std::env::var(CHILD) {
        // SAFETY: the buffer and output are live; the intentionally invalid
        // index must abort before any access, in this isolated child process.
        unsafe {
            let blob = bytes_of(&[1, 2, 3]);
            let mut out = 7;
            if operation == "at" {
                abi::vut_rt_bytes_at_v1(blob, 99, std::ptr::from_mut(&mut out));
            } else {
                abi::vut_rt_bytes_set_v1(blob, 99, 1);
            }
            abi::vut_rt_bytes_release_v1(blob);
        }
        panic!("invalid explicit index returned instead of trapping");
    }
    for operation in ["at", "set"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "bytes_explicit_oob_traps", "--nocapture"])
            .env(CHILD, operation)
            .output()
            .unwrap();
        assert!(!output.status.success(), "bytes.{operation} did not trap");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!(
                "bytes.{operation}: index 99 out of range (length 3)"
            )),
            "wrong failure: {stderr}"
        );
    }
}

#[test]
fn bytes_utf8_classification_reports_exact_locations() {
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
    // SAFETY: handles are created and released within the test.
    unsafe {
        let valid = bytes_of("Xin chào".as_bytes());
        assert_eq!(
            abi::vut_rt_bytes_utf8_valid_up_to_v1(valid),
            "Xin chào".len()
        );
        assert_eq!(abi::vut_rt_bytes_utf8_error_len_v1(valid), 0);
        let text = abi::vut_rt_bytes_to_str_v1(valid);
        assert!(!text.is_null());
        abi::vut_rt_release_string_v1(text);
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
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
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
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
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
    let _guard = BUFFER_TEST_LOCK.lock().unwrap();
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
