use crate::robots::from_user_agent_into_product_token::from_user_agent_into_product_token;
use crate::robots::robots::{Robots, Rule};

#[derive(Default)]
struct Group {
    agents: Vec<String>,
    rules: Vec<Rule>,
    crawl_delay: Option<f64>,
}

/// Parse `robots.txt` text for the crawler whose `User-Agent` is `user_agent`.
pub fn from_text_and_user_agent_into_robots(text: &str, user_agent: &str) -> Robots {
    let ours = from_user_agent_into_product_token(user_agent);

    let mut groups: Vec<Group> = Vec::new();
    let mut in_agent_run = false; // consecutive User-agent lines share a group
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else { continue };
        let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
        match key.as_str() {
            "user-agent" => {
                if !in_agent_run || groups.is_empty() {
                    groups.push(Group::default());
                }
                in_agent_run = true;
                groups.last_mut().unwrap().agents.push(value.to_ascii_lowercase());
            }
            "allow" | "disallow" => {
                in_agent_run = false;
                if let Some(g) = groups.last_mut() {
                    if !value.is_empty() {
                        g.rules.push(Rule {
                            allow: key == "allow",
                            pattern: value.to_string(),
                        });
                    }
                }
            }
            "crawl-delay" => {
                in_agent_run = false;
                if let (Some(g), Ok(d)) = (groups.last_mut(), value.parse::<f64>()) {
                    if d.is_finite() && d >= 0.0 {
                        g.crawl_delay = Some(d);
                    }
                }
            }
            _ => in_agent_run = false,
        }
    }

    let specific = |g: &Group| {
        g.agents
            .iter()
            .any(|a| a != "*" && !a.is_empty() && ours.contains(a.as_str()))
    };
    let wildcard = |g: &Group| g.agents.iter().any(|a| a == "*");
    let use_specific = groups.iter().any(specific);

    let mut robots = Robots::default();
    for g in groups
        .into_iter()
        .filter(|g| if use_specific { specific(g) } else { wildcard(g) })
    {
        robots.rules.extend(g.rules);
        robots.crawl_delay = match (robots.crawl_delay, g.crawl_delay) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
    }
    robots
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::url::Url;

    fn allowed(robots: &Robots, path: &str) -> bool {
        robots.is_allowed(&Url::parse(&format!("https://h{path}")).unwrap())
    }

    #[test]
    fn specific_group_beats_wildcard() {
        let text = "User-agent: *\nDisallow: /\n\nUser-agent: web-mirror\nDisallow: /secret/\n";
        let r = from_text_and_user_agent_into_robots(text, "web-mirror/0.1");
        assert!(allowed(&r, "/open"));
        assert!(!allowed(&r, "/secret/x"));
        // Another crawler gets the wildcard group.
        let other = from_text_and_user_agent_into_robots(text, "other-bot/1");
        assert!(!allowed(&other, "/open"));
    }

    #[test]
    fn stacked_user_agents_share_rules() {
        let r = from_text_and_user_agent_into_robots(
            "User-agent: foo\nUser-agent: web-mirror\nDisallow: /x\n",
            "web-mirror",
        );
        assert!(!allowed(&r, "/x"));
    }

    #[test]
    fn crawl_delay_and_comments() {
        let r = from_text_and_user_agent_into_robots(
            "# hi\nUser-agent: * # everyone\nCrawl-delay: 5\nDisallow: /a # no\n",
            "web-mirror",
        );
        assert_eq!(r.crawl_delay(), Some(5.0));
        assert!(!allowed(&r, "/a"));
    }
}
