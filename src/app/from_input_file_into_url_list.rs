use crate::error::error::Error;
use crate::url::from_str_into_start_url::from_str_into_start_url;
use ::url::Url;
use std::io::Read;
use std::path::Path;

/// Read start URLs for `-i`, `--input-file`: one per line, from `path`, or from
/// standard input when `path` is `-`. Blank lines and lines starting with `#`
/// are skipped. Every URL must be a valid start URL.
pub fn from_input_file_into_url_list(path: &Path) -> Result<Vec<Url>, Error> {
    let text = if path == Path::new("-") {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text).map_err(|e| Error::io(path, e))?;
        text
    } else {
        std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?
    };
    from_str_into_url_list(&text, path)
}

/// Parse the text of an input file; `path` is only used in error messages.
fn from_str_into_url_list(text: &str, path: &Path) -> Result<Vec<Url>, Error> {
    let mut urls = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let url = from_str_into_start_url(line)
            .map_err(|e| Error::Argument(format!("{}:{}: {e}", path.display(), index + 1)))?;
        urls.push(url);
    }
    Ok(urls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::temp_dir;

    #[test]
    fn skips_blanks_and_comments() {
        let urls = from_str_into_url_list(
            "# start pages\nhttps://h/a/\n\n  https://h/b/  \n#https://h/c/\n",
            Path::new("in.txt"),
        )
        .unwrap();
        let urls: Vec<String> = urls.into_iter().map(String::from).collect();
        assert_eq!(urls, ["https://h/a/", "https://h/b/"]);
    }

    #[test]
    fn bad_line_names_the_line() {
        let err = from_str_into_url_list("https://h/a/\nftp://h/\n", Path::new("in.txt")).unwrap_err();
        assert!(err.to_string().contains("in.txt:2"), "{err}");
    }

    #[test]
    fn reads_a_file() {
        let dir = temp_dir("input-file");
        let path = dir.join("urls.txt");
        std::fs::write(&path, "https://h/a/\n").unwrap();
        assert_eq!(from_input_file_into_url_list(&path).unwrap().len(), 1);
        assert!(from_input_file_into_url_list(&dir.join("missing.txt")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
