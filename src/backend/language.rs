use super::editor::GlobalSystemData;
use super::highlight::{Factory, Highlighter};
use super::syntax::tree_sitter::language::{Init, Language};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use toml::{de, Value};

pub struct Languages {
    languages: Vec<Box<dyn Factory>>,
    extensions: HashMap<String, usize>,
}

impl Default for Languages {
    fn default() -> Self {
        "
[javascript]
url = \"https://github.com/tree-sitter/tree-sitter-javascript\"
extensions = [
    \"js\"
]

[c]
url = \"https://github.com/tree-sitter/tree-sitter-c\"
extensions = [
    \"c\"
]
        ".parse().unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    Toml(de::Error),
    UrlIsNotString(Value),
    ExtensionsIsNotArray(Value),
    ExtensionIsNotString(Value),
    IllegalKey(String),
    LanguageDataIsNotTable(Value),
    LangDoesntHaveUrlAndExtensions,
    TomlIsntTable(Value)
}

impl FromStr for Languages {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let toml = s.parse::<Value>().map_err(Error::Toml)?;
        if let Value::Table(t) = toml {
            let mut data = Vec::new();
            for (lang, opts) in t {
                let mut url = None;
                let mut extensions = None;
                if let Value::Table(t) = opts {
                    for (k, v) in t {
                        match &k[..] {
                            "url" => {
                                if let Value::String(u) = v {
                                    url = Some(u);
                                } else {
                                    return Err(Error::UrlIsNotString(v)); // illegal
                                }
                            },
                            "extensions" => {
                                if let Value::Array(a) = v {
                                    let mut working_exts = Vec::new();
                                    for ext in a {
                                        working_exts.push(if let Value::String(e) = ext {
                                            e
                                        } else {
                                            return Err(Error::ExtensionIsNotString(ext)); // key in exts isn't a string
                                        });
                                    }
                                    extensions = Some(working_exts);
                                } else {
                                    return Err(Error::ExtensionsIsNotArray(v)); // illegal
                                }
                            },
                            k => return Err(Error::IllegalKey(k.to_string())) // illegal key _
                        }
                    }
                } else {
                    return Err(Error::LanguageDataIsNotTable(opts));
                }
                if let (Some(url), Some(exts)) = (url, extensions) {
                    data.push((lang, url, exts));
                } else {
                    return Err(Error::LangDoesntHaveUrlAndExtensions); // lang doesn't contain both a url and extensions
                }
            }

            let mut languages: Vec<Box<dyn Factory>> = Vec::new();
            let mut extensions = HashMap::new();
            for (lang, url, exts) in data {
                let idx = languages.len();
                languages.push(Box::new(Language::new(
                    Init::new(url, lang),
                    GlobalSystemData::default()
                )));
                for ext in exts {
                    extensions.insert(ext, idx);
                }
            }
            Ok(Languages {
                languages,
                extensions
            })
        } else {
            Err(Error::TomlIsntTable(toml))
        }
    }
}

impl Debug for Languages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Languages {{ extensions: {:?} }}", self.extensions)
    }
}

impl Languages {
    pub fn get(&self, extension: &str) -> anyhow::Result<Box<dyn Highlighter>> {
        let lang = self
            .extensions
            .get(extension.split('.').last().ok_or_else(|| UnknownExtension(String::from("[none]")))?)
            .ok_or_else(|| UnknownExtension(String::from(extension)))?;
        self.languages[*lang].make()
    }
}

#[derive(Debug)]
pub struct UnknownExtension(String);

impl StdError for UnknownExtension {}

impl Display for UnknownExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "extension {} is not known", self.0)
    }
}
