//! robots.txt: any text, user agent, and URL parse and match without
//! panicking (patterns with `*` and `$` must not blow up).
#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use url::Url;
use web_mirror::robots::from_text_and_user_agent_into_robots::from_text_and_user_agent_into_robots;
use web_mirror::robots::is_allowed::is_allowed;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    text: &'a str,
    user_agent: &'a str,
    path: &'a str,
}

fuzz_target!(|input: Input| {
    let robots = from_text_and_user_agent_into_robots(input.text, input.user_agent);
    let mut url = Url::parse("https://h/").unwrap();
    url.set_path(input.path);
    let _ = is_allowed(&robots, &url);
});
