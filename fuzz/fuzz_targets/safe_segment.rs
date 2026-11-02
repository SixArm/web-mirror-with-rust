//! Any string becomes one safe name, and a safe name stays as it is.
#![no_main]

use libfuzzer_sys::fuzz_target;
use web_mirror::url::from_str_into_is_safe_segment::from_str_into_is_safe_segment;
use web_mirror::url::from_str_into_safe_segment::from_str_into_safe_segment;

fuzz_target!(|s: &str| {
    let safe = from_str_into_safe_segment(s);
    assert!(from_str_into_is_safe_segment(&safe), "{s:?} became unsafe {safe:?}");
    // Making a safe name safe again changes nothing.
    assert_eq!(from_str_into_safe_segment(&safe), safe, "not idempotent for {s:?}");
});
