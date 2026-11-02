//! URL → local path: for any URL and layout, the path stays under the root,
//! every name is safe on Linux, macOS, and Windows, and nothing panics.
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::path::{Component, PathBuf};
use url::Url;
use web_mirror::url::from_mirror_layout_and_url_into_path_buf::from_mirror_layout_and_url_into_path_buf;
use web_mirror::url::from_str_into_is_safe_segment::from_str_into_is_safe_segment;
use web_mirror::url::mirror_layout::MirrorLayout;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    host_directories: bool,
    no_directories: bool,
    cut_dirs: u8,
    default_page: Option<&'a str>,
    /// A whole URL as a site might write it, or a path for `https://h/`.
    whole_url: bool,
    text: &'a str,
    query: Option<&'a str>,
}

fuzz_target!(|input: Input| {
    let url = if input.whole_url {
        let Ok(url) = Url::parse(input.text) else { return };
        url
    } else {
        let mut url = Url::parse("https://h/").unwrap();
        url.set_path(input.text);
        url.set_query(input.query);
        url
    };
    let layout = MirrorLayout {
        root: PathBuf::from("out"),
        host_directories: input.host_directories,
        cut_dirs: usize::from(input.cut_dirs % 8),
        default_page: input.default_page.unwrap_or("index.html").to_string(),
        no_directories: input.no_directories,
    };
    let path = from_mirror_layout_and_url_into_path_buf(&layout, &url);
    let rest = path.strip_prefix(&layout.root).expect("path starts with the root");
    let names: Vec<_> = rest.components().collect();
    assert!(!names.is_empty(), "no file name: {path:?}");
    for name in &names {
        let Component::Normal(name) = name else { panic!("not a plain name in {path:?}") };
        let name = name.to_str().expect("names are UTF-8");
        assert!(from_str_into_is_safe_segment(name), "unsafe name {name:?} in {path:?}");
    }
    assert_ne!(names[0].as_os_str(), ".web-mirror", "{path:?}");
});
