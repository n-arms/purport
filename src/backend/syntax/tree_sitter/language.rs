use super::highlighter::{HighlightQuery, IllegalQuery, TreeSitterHighlighter};
use super::loader;
use crate::backend::editor::GlobalSystemData;
use crate::backend::highlight::{Factory, Highlighter};
use once_cell::unsync::OnceCell;
use std::cell::Cell;
use std::error::Error as StdError;
use std::fmt::Display;
use std::rc::Rc;

use super::loader::Lib;
use libloading::Library;

#[derive(Debug)]
pub enum Error {
    Loader(loader::Error),
    Query(IllegalQuery),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loader(l) => write!(f, "failed to load a language with error: {:?}", l),
            Self::Query(q) => write!(f, "language has malformed query: {:?}", q),
        }
    }
}

impl StdError for Error {}

/// A loaded tree sitter parser
#[derive(Debug, Clone)]
pub struct Loaded {
    /// the data needed to highlight a tree.
    /// HighlightQuery contains a non-clonable Query, so an Rc is needed
    pub query: Rc<HighlightQuery>,
    pub language: tree_sitter::Language,
    _lib: Rc<Library>
}

/// A lazy loaded tree sitter parser
pub struct Language {
    cell: OnceCell<Result<Loaded, Error>>,
    init: Cell<Option<Box<dyn FnOnce() -> Result<Loaded, Error>>>>,
}

/// The data needed to initialize a Language
///
/// Since using tree sitter parsers basically involves downloading and running unsandboxed code from the
/// internet en masse, it would make sense to add a crypgraphic checksum field.
///
/// Additionally, in the future it might be worthwhile to add custom build scripts for parsers.
/// Many tree sitter parsers depend on more than just c and c++ (eg. the zig parser).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Init {
    /// The url to the git repo that the parser sits in. The repo should have a similar shape to parsers such as
    /// github.com/tree-sitter/tree-sitter-javascript, although it only needs the src/ and query/ folders
    git_repo: String,
    /// The name of the language (without the tree-sitter- prefix), for example "javascript"
    name: String,
}

impl Init {
    pub fn new(git_repo: String, name: String) -> Self {
        Init { git_repo, name }
    }
}

impl Factory for Language {
    fn make(&self) -> anyhow::Result<Box<dyn Highlighter>> {
        let l = self
            .cell
            .get_or_init(|| match self.init.take() {
                Some(f) => f(),
                None => panic!("lazy loaded language has been poisoned"),
            })
            .as_ref()
            .map_err(ToString::to_string)
            .map_err(StringError)?;
        let t = TreeSitterHighlighter::new(l.clone()).map_err(anyhow::Error::new)?;
        Ok(Box::new(t))
    }
}

impl Language {
    pub fn new(lang: Init, sys: GlobalSystemData) -> Self {
        Language {
            init: Cell::new(Some(Box::new(move || {
                let mut install_path = sys.target_dir.clone();
                install_path.push(format!("tree-sitter-{}/", lang.name));

                let mut compile_path = sys.target_dir.clone();
                compile_path.push(format!("tree-sitter-{}-build/", lang.name));

                Lib::install(&install_path, &lang.git_repo).map_err(Error::Loader)?;
                let l = Lib::build_lib(lang.name, compile_path, install_path, sys.cpp_compiler, sys.c_compiler).map_err(Error::Loader)?;
                let capture_table = TreeSitterHighlighter::make_capture_table(&l.highlighting).map_err(Error::Query)?;
                Ok(Loaded {
                    language: l.lang,
                    query: Rc::new(HighlightQuery::new(
                        l.highlighting,
                        capture_table
                    )),
                    _lib: Rc::new(l.lib)
                })
            }))),
            cell: OnceCell::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StringError(String);

impl Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for StringError {}
