use super::editor::GlobalSystemData;
use super::highlight::{Factory, Highlighter};
use super::syntax::tree_sitter::language::{Init, Language};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use toml::{de, Value};
use hex::{FromHexError, decode};
use std::convert::TryInto;

pub struct Languages {
    languages: Vec<Box<dyn Factory>>,
    extensions: HashMap<String, usize>,
}

impl Default for Languages {
    fn default() -> Self {
        "
[javascript]
url = \"https://github.com/tree-sitter/tree-sitter-javascript/archive/fdeb68ac8d2bd5a78b943528bb68ceda3aade2eb.zip\"
hash = \"12d976b625f2a439cadccd24cda0a39a61d956e0ab1557542e99eb178587b786\"
extensions = [
    \"js\"
]

[c]
url = \"https://github.com/tree-sitter/tree-sitter-c/archive/f05e279aedde06a25801c3f2b2cc8ac17fac52ae.zip\"
hash = \"0608ec6f4544aa851f0bdbb90698003a06709d9c087031e99dba875842a29281\"
extensions = [
    \"c\"
]
        "
        .parse()
        .unwrap()
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
    TomlIsntTable(Value),
    Hex(FromHexError),
    WrongHexSize
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
                let mut hash = None;
                if let Value::Table(t) = opts {
                    for (k, v) in t {
                        match &k[..] {
                            "url" => {
                                if let Value::String(u) = v {
                                    url = Some(u);
                                } else {
                                    return Err(Error::UrlIsNotString(v)); // illegal
                                }
                            }
                            "hash" => {
                                if let Value::String(h) = v {
                                    hash = Some(decode(h).map_err(Error::Hex)?);
                                } else {
                                    return Err(Error::UrlIsNotString(v)); // illegal
                                }
                            }
                            "extensions" => {
                                if let Value::Array(a) = v {
                                    let mut working_exts = Vec::new();
                                    for ext in a {
                                        working_exts.push(if let Value::String(e) = ext {
                                            e
                                        } else {
                                            return Err(Error::ExtensionIsNotString(ext));
                                            // key in exts isn't a string
                                        });
                                    }
                                    extensions = Some(working_exts);
                                } else {
                                    return Err(Error::ExtensionsIsNotArray(v)); // illegal
                                }
                            }
                            k => return Err(Error::IllegalKey(k.to_string())), // illegal key _
                        }
                    }
                } else {
                    return Err(Error::LanguageDataIsNotTable(opts));
                }
                if let (Some(url), Some(exts), Some(hash)) = (url, extensions, hash) {
                    data.push((lang, url, hash, exts));
                } else {
                    return Err(Error::LangDoesntHaveUrlAndExtensions); // lang doesn't contain both a url and extensions
                }
            }

            let mut languages: Vec<Box<dyn Factory>> = Vec::new();
            let mut extensions = HashMap::new();
            for (lang, url, hash, exts) in data {
                let idx = languages.len();
                languages.push(Box::new(Language::new(
                    Init::new(url, lang, TryInto::try_into(hash).map_err(|_| Error::WrongHexSize)?),
                    GlobalSystemData::default(),
                )));
                for ext in exts {
                    extensions.insert(ext, idx);
                }
            }
            Ok(Languages {
                languages,
                extensions,
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
            .get(
                extension
                    .split('.')
                    .last()
                    .ok_or_else(|| UnknownExtension(String::from("[none]")))?,
            )
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
