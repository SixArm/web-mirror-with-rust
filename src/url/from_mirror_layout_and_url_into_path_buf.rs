use crate::url::from_str_into_safe_segment::from_str_into_safe_segment;
use crate::url::from_url_and_default_page_into_dir_segments_and_file_name::from_url_and_default_page_into_dir_segments_and_file_name;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::PathBuf;

/// Map a URL to the local file that mirrors it.
///
/// With the default layout:
///
/// * `https://h/a/b/`   → `root/h/a/b/index.html`
/// * `https://h/a/b.pdf` → `root/h/a/b.pdf`
/// * `https://h/a/b`    → `root/h/a/b/index.html` (no extension: a page)
/// * query `?x=1`       → appended to the file stem: `index@x=1.html`
///
/// `--no-host-directories` drops the `h/` directory, `--cut-dirs=N` drops the
/// first N directories of the path, `--default-page` replaces `index.html`,
/// and `--no-directories` keeps only the file name (`root/b.pdf`; clashes
/// between URLs are numbered by the crawl, not here).
///
/// Segments are not percent-decoded, and unsafe characters become `_`, so a
/// URL can never write outside `root`.
pub fn from_mirror_layout_and_url_into_path_buf(layout: &MirrorLayout, url: &Url) -> PathBuf {
    let mut out = layout.root.clone();
    if layout.host_directories && !layout.no_directories {
        // The port is added before the name is made safe, so a long host
        // name with a port is still cut to a safe length (found by fuzzing).
        let host = url.host_str().unwrap_or("unknown-host");
        let host = match url.port() {
            Some(port) => from_str_into_safe_segment(&format!("{host}_{port}")),
            None => from_str_into_safe_segment(host),
        };
        out.push(host);
    }

    let (segments, file) = from_url_and_default_page_into_dir_segments_and_file_name(url, &layout.default_page);
    let mut file = from_str_into_safe_segment(&file);
    if let Some(query) = url.query() {
        let (stem, ext) = match file.rsplit_once('.') {
            Some((s, e)) => (s.to_string(), format!(".{e}")),
            None => (file.clone(), String::new()),
        };
        file = from_str_into_safe_segment(&format!("{stem}@{query}{ext}"));
    }
    let mut parts: Vec<String> = if layout.no_directories {
        Vec::new()
    } else {
        segments.iter().skip(layout.cut_dirs).map(|s| from_str_into_safe_segment(s)).collect()
    };
    parts.push(file);
    // web-mirror keeps its own files in `root/.web-mirror/`. With host
    // directories every URL is under `root/<host>/`, so it cannot clash; without
    // them, a site's own `/.web-mirror/...` is renamed so it can never
    // overwrite web-mirror's files.
    if (!layout.host_directories || layout.no_directories) && parts[0] == crate::disk::saved_record::STATE_DIR {
        parts[0].insert(0, '_');
    }
    out.extend(parts);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> MirrorLayout {
        MirrorLayout { root: PathBuf::from("out"), ..MirrorLayout::default() }
    }
    fn p(layout: &MirrorLayout, u: &str) -> PathBuf {
        from_mirror_layout_and_url_into_path_buf(layout, &Url::parse(u).unwrap())
    }

    #[test]
    fn default_layout() {
        let l = layout();
        assert_eq!(p(&l, "https://www.nhs.uk/conditions/"), PathBuf::from("out/www.nhs.uk/conditions/index.html"));
        assert_eq!(
            p(&l, "https://www.nhs.uk/conditions/acne"),
            PathBuf::from("out/www.nhs.uk/conditions/acne/index.html")
        );
        assert_eq!(p(&l, "https://h/a/b.pdf"), PathBuf::from("out/h/a/b.pdf"));
        assert_eq!(p(&l, "http://h:8080/"), PathBuf::from("out/h_8080/index.html"));
    }

    #[test]
    fn no_directories_keeps_only_the_file_name() {
        let l = MirrorLayout { no_directories: true, ..layout() };
        assert_eq!(p(&l, "https://h/a/b/c.pdf"), PathBuf::from("out/c.pdf"));
        assert_eq!(p(&l, "https://h/a/b/"), PathBuf::from("out/index.html"));
        assert_eq!(p(&l, "https://h/a/?x=1"), PathBuf::from("out/index@x=1.html"));
        // Never web-mirror's own state directory.
        assert_eq!(p(&l, "https://h/a/.web-mirror"), PathBuf::from("out/_.web-mirror"));
    }

    #[test]
    fn query_goes_before_extension() {
        let l = layout();
        assert_eq!(p(&l, "https://h/a/?p=2"), PathBuf::from("out/h/a/index@p=2.html"));
        assert_eq!(p(&l, "https://h/a/b.pdf?v=1"), PathBuf::from("out/h/a/b@v=1.pdf"));
    }

    #[test]
    fn no_host_directories() {
        let l = MirrorLayout { host_directories: false, ..layout() };
        assert_eq!(p(&l, "https://h/a/b.pdf"), PathBuf::from("out/a/b.pdf"));
    }

    #[test]
    fn never_clashes_with_the_state_directory() {
        let l = MirrorLayout { host_directories: false, ..layout() };
        assert_eq!(p(&l, "https://h/.web-mirror/saved.tsv"), PathBuf::from("out/_.web-mirror/saved.tsv"));
        let l = MirrorLayout { host_directories: false, cut_dirs: 1, ..layout() };
        assert_eq!(p(&l, "https://h/a/.web-mirror/saved.tsv"), PathBuf::from("out/_.web-mirror/saved.tsv"));
        assert_eq!(p(&layout(), "https://h/.web-mirror/saved.tsv"), PathBuf::from("out/h/.web-mirror/saved.tsv"));
    }

    #[test]
    fn cut_dirs() {
        let l = MirrorLayout { cut_dirs: 1, ..layout() };
        assert_eq!(p(&l, "https://h/a/b/c.pdf"), PathBuf::from("out/h/b/c.pdf"));
        assert_eq!(p(&l, "https://h/a/"), PathBuf::from("out/h/index.html"));
        let l = MirrorLayout { cut_dirs: 9, host_directories: false, ..layout() };
        assert_eq!(p(&l, "https://h/a/b/c.pdf"), PathBuf::from("out/c.pdf"));
    }

    #[test]
    fn default_page() {
        let l = MirrorLayout { default_page: "default.htm".into(), ..layout() };
        assert_eq!(p(&l, "https://h/a/"), PathBuf::from("out/h/a/default.htm"));
        assert_eq!(p(&l, "https://h/a/?x=1"), PathBuf::from("out/h/a/default@x=1.htm"));
    }

    #[test]
    fn long_host_with_port_is_still_a_safe_name() {
        let host = "a".repeat(250);
        let path = p(&layout(), &format!("https://{host}:8443/x.pdf"));
        let name = path.iter().nth(1).unwrap().to_str().unwrap();
        assert!(crate::url::from_str_into_is_safe_segment::from_str_into_is_safe_segment(name), "{name}");
    }

    #[test]
    fn cannot_escape_root() {
        let path = p(&layout(), "https://h/a/%2e%2e/%2e%2e/etc/passwd.txt");
        assert!(path.starts_with("out/h"));
        assert!(!path.components().any(|c| c.as_os_str() == ".."));
    }

    /// Property tests: for arbitrary URLs and layouts the path never leaves
    /// the root and every name is safe on Linux, macOS, and Windows.
    mod properties {
        use super::*;
        use crate::url::from_str_into_safe_segment::tests::assert_safe_segment;
        use proptest::prelude::*;

        /// Path text built from pieces that have caused trouble in other
        /// tools: dot segments (plain and escaped), separators, device names,
        /// trailing dots and spaces, long names, and non-ASCII.
        fn path_text() -> impl Strategy<Value = String> {
            let piece = prop_oneof![
                Just("/".to_string()),
                Just(".".to_string()),
                Just("..".to_string()),
                Just("%2e".to_string()),
                Just("%2E%2e".to_string()),
                Just("%2f".to_string()),
                Just("%00".to_string()),
                Just("\\".to_string()),
                Just(".web-mirror".to_string()),
                Just("CON".to_string()),
                Just("nul.txt".to_string()),
                Just(" ".to_string()),
                Just("a".repeat(250)),
                "[a-zA-Z0-9._~ -]{0,12}",
                any::<String>(),
            ];
            prop::collection::vec(piece, 0..12).prop_map(|v| v.concat())
        }

        fn layout_strategy() -> impl Strategy<Value = MirrorLayout> {
            (any::<bool>(), 0usize..4, prop_oneof![Just("index.html".to_string()), any::<String>()], any::<bool>())
                .prop_map(|(host_directories, cut_dirs, default_page, no_directories)| MirrorLayout {
                    root: PathBuf::from("out").join("root"),
                    host_directories,
                    cut_dirs,
                    default_page,
                    no_directories,
                })
        }

        fn url_strategy() -> impl Strategy<Value = Url> {
            (
                prop_oneof![
                    Just("h".to_string()),
                    "[a-z0-9.-]{1,20}",
                    "[a-z]{190,260}",
                    Just("CON".to_string()),
                    Just("[::1]".to_string())
                ],
                proptest::option::of(1u16..),
                path_text(),
                proptest::option::of(path_text()),
            )
                .prop_filter_map("not a URL", |(host, port, path, query)| {
                    let mut url = Url::parse(&format!("https://{host}/")).ok()?;
                    url.set_port(port).ok()?;
                    url.set_path(&path);
                    url.set_query(query.as_deref());
                    Some(url)
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(10_000))]

            #[test]
            fn path_stays_under_root_with_safe_names(layout in layout_strategy(), url in url_strategy()) {
                let path = from_mirror_layout_and_url_into_path_buf(&layout, &url);
                let rest = path.strip_prefix(&layout.root).expect("path starts with the root");
                let names: Vec<_> = rest.components().collect();
                prop_assert!(!names.is_empty(), "no file name: {path:?}");
                for name in &names {
                    let std::path::Component::Normal(name) = name else {
                        return Err(TestCaseError::fail(format!("not a plain name in {path:?}")));
                    };
                    assert_safe_segment(name.to_str().expect("names are UTF-8"));
                }
                // web-mirror's own directory is never a mirror target.
                prop_assert_ne!(names[0].as_os_str(), crate::disk::saved_record::STATE_DIR);
                if layout.no_directories {
                    prop_assert_eq!(names.len(), 1);
                }
            }
        }
    }
}
