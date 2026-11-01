// Data structures
#[macro_use]
pub(crate) mod list; //use crate::types::list::*;
#[macro_use]
pub(crate) mod map; //use crate::types::map::*;
#[macro_use]
pub(crate) mod queue; //use crate::types::queue::*;
#[macro_use]
pub(crate) mod set; //use crate::types::set::*;
#[macro_use]
pub(crate) mod stack; //use crate::types::stack::*;

// Semantic strings
#[macro_use]
pub(crate) mod glob_str; //use crate::types::glob_str::glob_str;
#[macro_use]
pub(crate) mod glob_string; //use crate::types::glob_string::glob_string;
#[macro_use]
pub(crate) mod html_str; //use crate::types::html_str::html_str;
#[macro_use]
pub(crate) mod html_string; //use crate::types::html_string::html_string;
#[macro_use]
pub(crate) mod markdown_str; //use crate::types::markdown::*;
#[macro_use]
pub(crate) mod pathable_str; //use crate::types::pathable::*;
#[macro_use]
pub(crate) mod toml_str; //use crate::types::toml::*;
#[macro_use]
pub(crate) mod url_str; //use crate::types::url::*;
#[macro_use]
pub(crate) mod yaml_str; //use crate::types::yaml::*;

// Semantic strings (owned `String` variants).
pub(crate) mod json_str; //use crate::types::json_str::*;
pub(crate) mod json_string; //use crate::types::json_string::*;
pub(crate) mod markdown_string; //use crate::types::markdown_string::*;
pub(crate) mod pathable_string; //use crate::types::pathable_string::*;
pub(crate) mod toml_string; //use crate::types::toml_string::*;
pub(crate) mod url_string; //use crate::types::url_string::*;
pub(crate) mod yaml_string; //use crate::types::yaml_string::*;
