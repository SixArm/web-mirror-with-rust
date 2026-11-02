use crate::app::config::Config;
use crate::error::error::Error;
use ::clap::{ArgAction, Command};
use serde_json::Value;

/// Options a config file may not set: the config file itself, the start URL
/// (give it on the command line, or use `input-file`), and the
/// informational ones.
const NOT_IN_CONFIG: [&str; 4] = ["config", "help", "version", "test"];

/// Turn `config`'s options into command line arguments for `command`, to go
/// before the real ones, so that the command line overrides the file: a
/// later single-valued option replaces an earlier one, and repeatable
/// options (`--header`, `--accept`, ...) add to the file's values.
///
/// * Keys are long option names; `_` may be used for `-`.
/// * `true` gives a flag (`wait-jitter = true` → `--wait-jitter`); `false`
///   gives nothing. `verbose` takes a count (`verbose = 3` → `-vvv`).
/// * Strings and numbers give `--name=VALUE`.
/// * Arrays, for repeatable options only, give `--name=VALUE` for each item.
///
/// Unknown keys, nested tables, and the wrong kind of value are errors, so a
/// typo is never silently ignored.
pub fn from_config_and_command_into_arg_list(config: &Config, command: &Command) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    for (key, value) in &config.options {
        let name = key.replace('_', "-");
        let bad = |why: &str| Error::Argument(format!("config key {key:?}: {why}"));
        if NOT_IN_CONFIG.contains(&name.as_str()) {
            return Err(bad("cannot be set in a config file"));
        }
        let arg = command
            .get_arguments()
            .find(|a| a.get_long() == Some(name.as_str()))
            .ok_or_else(|| bad("not a web-mirror option"))?;
        match arg.get_action() {
            ArgAction::SetTrue => match value {
                Value::Bool(true) => out.push(format!("--{name}")),
                Value::Bool(false) => {}
                _ => return Err(bad("must be true or false")),
            },
            ArgAction::Count => {
                let n = match value {
                    Value::Bool(b) => u64::from(*b),
                    Value::Number(n) => n.as_u64().filter(|&n| n <= 5).ok_or_else(|| bad("must be 0 to 5"))?,
                    _ => return Err(bad("must be a number")),
                };
                out.extend(std::iter::repeat_n(format!("--{name}"), n as usize));
            }
            ArgAction::Append => match value {
                Value::Array(items) => {
                    for item in items {
                        out.push(format!(
                            "--{name}={}",
                            scalar(item).ok_or_else(|| bad("items must be strings or numbers"))?
                        ));
                    }
                }
                other => out.push(format!(
                    "--{name}={}",
                    scalar(other).ok_or_else(|| bad("must be a string, number, or array"))?
                )),
            },
            _ => out.push(format!("--{name}={}", scalar(value).ok_or_else(|| bad("must be a string or number"))?)),
        }
    }
    Ok(out)
}

/// A string or number as option text.
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::clap::app;

    fn config(toml: &str) -> Config {
        let dir = crate::app::testing::temp_dir("config-args");
        let path = dir.join("c.toml");
        std::fs::write(&path, toml).unwrap();
        let config = ::confy::load_path(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        config
    }

    #[test]
    fn values_become_arguments() {
        let c = config(
            "wait = 2\nwait_jitter = true\nforce = false\nverbose = 3\nreject = [\"mp4\", \"zip\"]\nuser-agent = \"bot/1\"\n",
        );
        let args = from_config_and_command_into_arg_list(&c, &app()).unwrap();
        assert_eq!(
            args,
            [
                "--reject=mp4",
                "--reject=zip",
                "--user-agent=bot/1",
                "--verbose",
                "--verbose",
                "--verbose",
                "--wait=2",
                "--wait-jitter"
            ]
        );
    }

    #[test]
    fn mistakes_are_errors() {
        for toml in [
            "wiat = 2\n",
            "config = \"other.toml\"\n",
            "test = true\n",
            "wait-jitter = \"yes\"\n",
            "wait = [1, 2]\n",
            "verbose = 9\n",
            "header = { a = 1 }\n",
        ] {
            assert!(from_config_and_command_into_arg_list(&config(toml), &app()).is_err(), "{toml}");
        }
    }
}
