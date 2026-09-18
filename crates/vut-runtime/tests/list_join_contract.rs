use vut_runtime::abi;

#[test]
fn list_join_concatenates_and_balances_string_references() {
    let strings_baseline = abi::live_string_allocations();
    let lists_baseline = abi::live_list_allocations();

    // SAFETY: every handle is created by the runtime, remains live while
    // borrowed, and each owned reference is released exactly once.
    unsafe {
        let list = abi::vut_rt_list_new_v1(
            8,
            8,
            abi::vut_rt_slot_retain_string_v1 as *const (),
            abi::vut_rt_slot_release_string_v1 as *const (),
            0,
        );
        for text in ["a", "b", "c"] {
            let handle = abi::managed_string(text);
            abi::vut_rt_list_push_v1(list, std::ptr::addr_of!(handle).cast::<u8>());
            abi::vut_rt_release_string_v1(handle);
        }
        let separator = abi::managed_string("-");
        let joined = abi::vut_rt_list_join_v1(list, separator);
        assert_eq!(abi::string_value(joined).unwrap().as_str(), "a-b-c");

        let single = abi::vut_rt_list_new_v1(
            8,
            8,
            abi::vut_rt_slot_retain_string_v1 as *const (),
            abi::vut_rt_slot_release_string_v1 as *const (),
            0,
        );
        let empty = abi::vut_rt_list_join_v1(single, separator);
        assert_eq!(abi::string_value(empty).unwrap().as_str(), "");

        abi::vut_rt_release_string_v1(empty);
        abi::vut_rt_release_string_v1(joined);
        abi::vut_rt_release_string_v1(separator);
        abi::vut_rt_list_release_v1(single);
        abi::vut_rt_list_release_v1(list);
    }

    assert_eq!(abi::live_string_allocations(), strings_baseline);
    assert_eq!(abi::live_list_allocations(), lists_baseline);
}
