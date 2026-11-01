use crate::url::from_mirror_layout_and_url_into_path_buf::from_mirror_layout_and_url_into_path_buf;
use crate::url::mirror_layout::MirrorLayout;
use ::url::Url;
use std::path::{Path, PathBuf};

/// Map a URL to `root/host/path...` with the default layout.
///
/// See [`from_mirror_layout_and_url_into_path_buf`] for the mapping rules and
/// for the layout options.
pub fn from_root_and_url_into_mirror_path_buf(root: &Path, url: &Url) -> PathBuf {
    let layout = MirrorLayout { root: root.to_path_buf(), ..MirrorLayout::default() };
    from_mirror_layout_and_url_into_path_buf(&layout, url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_url() {
        let url = Url::parse("https://www.nhs.uk/conditions/").unwrap();
        assert_eq!(
            from_root_and_url_into_mirror_path_buf(Path::new("out"), &url),
            PathBuf::from("out/www.nhs.uk/conditions/index.html")
        );
    }
}
