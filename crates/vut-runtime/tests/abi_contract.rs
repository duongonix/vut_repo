use vut_runtime::abi;

#[test]
fn versioned_string_abi_obeys_null_and_ownership_contracts() {
    assert_eq!(abi::VERSION, 13);
    let baseline = abi::live_string_allocations();

    // SAFETY: handles are created by the runtime, remain live while borrowed,
    // and each owned reference is released exactly once.
    unsafe {
        assert_eq!(abi::vut_rt_string_byte_len_v1(std::ptr::null()), 0);
        assert_eq!(abi::vut_rt_string_is_empty_v1(std::ptr::null()), 1);
        assert!(abi::vut_rt_string_trim_v1(std::ptr::null()).is_null());

        let text = abi::vut_rt_string_from_utf8_v1(" Việt ".as_ptr(), 8);
        let needle = abi::vut_rt_string_from_utf8_v1("Việt".as_ptr(), 6);
        assert!(!text.is_null());
        assert_eq!(abi::vut_rt_string_byte_len_v1(text), 8);
        assert_eq!(abi::vut_rt_string_char_len_v1(text), 6);
        assert_eq!(abi::vut_rt_string_contains_v1(text, needle), 1);

        abi::vut_rt_retain_string_v1(text);
        abi::vut_rt_release_string_v1(text);
        abi::vut_rt_release_string_v1(needle);
        abi::vut_rt_release_string_v1(text);
    }
    assert_eq!(abi::live_string_allocations(), baseline);
    assert!((abi::vut_rt_f64_mod_v1(7.5, 2.0) - 1.5).abs() < f64::EPSILON);
}
