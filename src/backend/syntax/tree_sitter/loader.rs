use libloading::{Library, Symbol};

use std::ffi::OsStr;

use reqwest::blocking::get;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use tree_sitter::{Language, Query, QueryError};
use zip::read::ZipArchive;

#[cfg(unix)]
const DYN_LIB_EXT: &str = "so";

#[cfg(windows)]
const DYN_LIB_EXT: &'static str = "dll";

#[derive(Debug)]
pub struct Lib {
    pub lib: Library,
    pub lang: Language,
    pub highlighting: Query,
}

impl Lib {
    fn files_with_ext(dir: impl AsRef<Path>, ext: impl AsRef<str>) -> Vec<Box<Path>> {
        if let Ok(files) = fs::read_dir(dir) {
            files
                .filter_map(|file| {
                    if let Ok(file) = file {
                        let path = file.path().into_boxed_path();
                        if path.extension().and_then(OsStr::to_str) == Some(ext.as_ref()) {
                            Some(path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn compile_to_obj(
        compiler: impl AsRef<str>,
        include_dir: impl AsRef<Path>,
        file: impl AsRef<Path> + Clone,
        object_path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let comp_status = Command::new(compiler.as_ref())
            .arg("-fPIC")
            .arg("-c")
            .arg("-I")
            .arg(include_dir.as_ref())
            .arg("-o")
            .arg(object_path.as_ref())
            .arg(&file.as_ref().to_str().ok_or_else(|| {
                Error::IllegalFileName(file.as_ref().to_path_buf().into_boxed_path())
            })?)
            .spawn()
            .map_err(Error::IO)?
            .wait()
            .map_err(Error::IO)?;
        if comp_status.success() {
            Ok(())
        } else {
            Err(Error::NonZeroExitStatus(
                comp_status.code(),
                compiler.as_ref().to_string(),
            ))
        }
    }

    /// given a c++ compiler (such as g++), a c compiler (such as gcc), the root directory of a tree sitter repo, the name of
    /// the language and a working directory, compile the tree sitter parser into a dynamic library
    /// and return the path
    fn compile_dyn_lib(
        cpp_compiler: impl AsRef<str>,
        c_compiler: impl AsRef<str>,
        root_dir: impl AsRef<Path>,
        lang_name: impl AsRef<str>,
        target_dir: impl AsRef<Path>,
    ) -> Result<impl AsRef<Path>, Error> {
        if !Lib::command_succeeds(&format!("{} --version", c_compiler.as_ref())) {
            return Err(Error::MissingPrerequisite(c_compiler.as_ref().to_string()));
        }
        if !Lib::command_succeeds(&format!("{} --version", cpp_compiler.as_ref())) {
            return Err(Error::MissingPrerequisite(
                cpp_compiler.as_ref().to_string(),
            ));
        }
        let mut src = root_dir.as_ref().to_path_buf();
        src.push("src/");
        let cc = Lib::files_with_ext(&src, "cc");
        let cpp = Lib::files_with_ext(&src, "cpp");
        let c = Lib::files_with_ext(&src, "c");
        for file in cc.into_iter().chain(cpp.into_iter()) {
            let mut object_path = target_dir.as_ref().to_path_buf();
            object_path.push(
                file.file_stem()
                    .expect("logic error in Lib::files_with_ext, produced file with no prefix"),
            );
            object_path.set_extension("o");
            Lib::compile_to_obj(
                &cpp_compiler,
                &src,
                file.to_str()
                    .ok_or_else(|| Error::IllegalFileName(file.clone()))?,
                object_path,
            )?;
        }
        for file in c {
            let mut object_path = target_dir.as_ref().to_path_buf();
            object_path.push(
                file.file_stem()
                    .expect("logic error in Lib::files_with_ext, produced file with no prefix"),
            );
            object_path.set_extension("o");
            Lib::compile_to_obj(
                &c_compiler,
                &src,
                file.to_str()
                    .ok_or_else(|| Error::IllegalFileName(file.clone()))?,
                object_path,
            )?;
        }

        let mut dyn_lib_name = String::from("lib");
        dyn_lib_name.push_str(lang_name.as_ref());

        let mut dyn_lib_path = target_dir.as_ref().to_path_buf();
        dyn_lib_path.push(dyn_lib_name);
        dyn_lib_path.set_extension(DYN_LIB_EXT);

        let mut dyn_link_cmd = Command::new(cpp_compiler.as_ref());
        dyn_link_cmd
            .arg("-shared")
            .arg("-o")
            .arg(&dyn_lib_path)
            .arg("-I")
            .arg(src);

        for file in Lib::files_with_ext(target_dir, "o") {
            dyn_link_cmd.arg(file.as_os_str());
        }

        let link_status = dyn_link_cmd
            .spawn()
            .map_err(Error::IO)?
            .wait()
            .map_err(Error::IO)?;

        if !link_status.success() {
            return Err(Error::NonZeroExitStatus(
                link_status.code(),
                cpp_compiler.as_ref().to_string(),
            ));
        }

        if dyn_lib_path.exists() {
            Ok(dyn_lib_path)
        } else {
            Err(Error::UnexpectedBehaviourFrom(
                cpp_compiler.as_ref().to_string(),
            ))
        }
    }

    pub fn build_lib(
        lang_name: impl AsRef<str>,
        target_dir: impl AsRef<Path>,
        root_dir: impl AsRef<Path>,
        cpp_compiler: impl AsRef<str>,
        c_compiler: impl AsRef<str>,
    ) -> Result<Lib, Error> {
        if !target_dir.as_ref().exists() {
            fs::create_dir_all(&target_dir).map_err(Error::IO)?;
        }
        let lib_path =
            Lib::compile_dyn_lib(cpp_compiler, c_compiler, &root_dir, &lang_name, target_dir)?;
        let lib = unsafe { Library::new(lib_path.as_ref()) }.map_err(Error::Linker)?;

        let mut lang_func_name = b"tree_sitter_".to_vec();
        lang_func_name.extend(lang_name.as_ref().bytes());

        let lang_sym: Symbol<fn() -> Language> =
            unsafe { lib.get(&lang_func_name) }.map_err(Error::Linker)?;
        let lang = lang_sym();

        let mut highlight_path = root_dir.as_ref().to_path_buf();
        highlight_path.push("queries/");
        highlight_path.push("highlights.scm");
        let highlighting = Query::new(
            lang,
            &fs::read_to_string(highlight_path).map_err(Error::IO)?,
        )
        .map_err(Error::MalformedQuery)?;

        Ok(Lib {
            lib,
            lang,
            highlighting,
        })
    }

    pub fn install(root_dir: impl AsRef<Path>, url: &str, hash: &[u8]) -> Result<PathBuf, Error> {
        if !root_dir.as_ref().exists() {
            fs::create_dir_all(root_dir.as_ref()).map_err(Error::IO)?;
        }
        let zipped_repo = get(url)
            .map_err(Error::Reqwest)?
            .bytes()
            .map_err(Error::Reqwest)?
            .to_vec();
        let repo_hash = Sha256::new().chain_update(&zipped_repo).finalize();
        if &repo_hash[..] != hash {
            return Err(Error::SecurityFlawDetected(
                url.to_string(),
                hash.to_vec(),
                repo_hash.to_vec(),
            ));
        }

        let mut install_dir = root_dir.as_ref().to_path_buf();
        let mut archive = ZipArchive::new(io::Cursor::new(&zipped_repo[..])).map_err(Error::Zip)?;
        archive
            .file_names()
            .min_by_key(|e| e.len())
            .map(|e| install_dir.push(e))
            .ok_or(Error::GitRepoWasEmpty)?;
        archive.extract(root_dir).map_err(Error::Zip)?;
        Ok(install_dir)
    }

    fn command_succeeds(cmd: &str) -> bool {
        let child = Command::new("sh").arg("-c").arg(cmd).spawn();
        if let Ok(exit) = child.and_then(|mut c| c.wait()) {
            exit.success()
        } else {
            false
        }
    }
}
/*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanuageDataBase {
    languages: HashMap<String, LanguageData>,
    extensions: HashMap<String, String>
}

impl LanuageDataBase {
    pub fn new(languages: HashMap<String, LanguageData>, extensions: HashMap<String, String>) -> Self {
        LanuageDataBase {
            languages,
            extensions
        }
    }

    pub fn add_extension(&mut self, extension: String, language: String) -> Option<String> {
        self.extensions.insert(extension, language)
    }

    pub fn get(&mut self, extension: &str) -> Option<&LanguageData> {
        let lang = self.extensions.get(extension)?;
        self.languages.get(lang)
    }
}
*/

#[derive(Debug)]
pub enum Error {
    Linker(libloading::Error),
    IO(io::Error),
    IllegalFileName(Box<Path>),
    NonZeroExitStatus(Option<i32>, String),
    UnexpectedBehaviourFrom(String),
    MalformedQuery(QueryError),
    MissingPrerequisite(String),
    Reqwest(reqwest::Error),
    SecurityFlawDetected(String, Vec<u8>, Vec<u8>),
    Zip(zip::result::ZipError),
    GitRepoWasEmpty,
}
