/// How to normalize query strings before URLs are queued or compared, so one
/// page is not fetched twice under two spellings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryRules {
    /// `--strip-query-params`: parameter names to remove; `*` and `?` are
    /// wildcards, such as `utm_*`.
    pub strip: Vec<String>,
    /// `--sort-query`: sort the parameters by name (then value).
    pub sort: bool,
    /// `--ignore-query`: drop the query entirely.
    pub ignore: bool,
}
