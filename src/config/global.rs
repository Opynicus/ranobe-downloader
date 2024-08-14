use once_cell::sync::Lazy;

use super::{Config, Template};
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::default().load().unwrap());

pub static TEMPLATE: Lazy<Template> = Lazy::new(|| Template::default().load().unwrap());
