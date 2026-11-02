//! URL helpers: parsing, and URL → local file path mapping.
pub mod from_js_str_and_url_into_url_list; // URLs in a JavaScript file (--follow-js).
pub mod from_js_str_into_url_str_list; // URL-looking strings in JavaScript.
pub mod from_mirror_layout_and_url_into_path_buf; // URL → local file (the main mapping).
pub mod from_path_and_content_type_into_adjusted_path_buf; // -E: add .html or .css.
pub mod from_path_and_number_into_numbered_path_buf; // index.html → index-1.html, for -nd clashes.
pub mod from_path_into_slash_string; // A relative path with `/` on every platform.
pub mod from_root_and_url_into_mirror_path_buf; // The same, with the default layout.
pub mod from_str_into_is_safe_segment; // Check a name is safe (for tests and fuzzing).
pub mod from_str_into_safe_segment; // Make one file or directory name safe.
pub mod from_str_into_start_url; // Parse a start URL as typed in a shell.
pub mod from_url_and_default_page_into_dir_segments_and_file_name; // Split a URL path.
pub mod from_url_and_query_rules_into_normalized_url; // One spelling per URL.
pub mod from_url_host_segments_into_path_buf; // URL → host/segments path (helper).
pub mod from_url_into_redacted_url; // Hide credentials before logging a URL.
pub mod from_url_str_list_and_base_url_into_url_list; // Resolve links against a base.
pub mod mirror_layout; // `MirrorLayout`: how files are laid out on disk.
pub mod query_rules; // `QueryRules`: --strip-query-params, --sort-query, --ignore-query.
