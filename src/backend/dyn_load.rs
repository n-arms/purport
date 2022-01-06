use libloading::{Library, Symbol};
use std::path::*;
use tree_sitter::{Language, Query, QueryError};
use std::io;
use std::process::Command;
use std::fs;
use std::ffi::OsStr;
use std::collections::HashMap;

#[cfg(unix)]
const DYN_LIB_EXT: &'static str = "so";

#[cfg(windows)]
const DYN_LIB_EXT: &'static str = "dll";

#[derive(Debug)]
pub struct Lib {
    pub lib: Library,
    pub lang: Language,
    pub highlighting: Query
}

#[derive(Debug)]
pub struct Libraries {
    /// the list of all loaded libraries and their language names
    libs: HashMap<String, Lib>,
}

impl Libraries {
    pub fn new() -> Self {
        Self {libs: HashMap::new()}
    }

    fn files_with_ext(dir: impl AsRef<Path>, ext: impl AsRef<str>) -> Vec<Box<Path>> {
        if let Ok(files) = fs::read_dir(dir) {
            files.filter_map(|file| if let Ok(file) = file {
                let path = file.path().into_boxed_path();
                if path.extension().and_then(|s| s.to_str()) == Some(ext.as_ref()) { 
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            }).collect()
        } else {
            Vec::new()
        }
    }

    fn compile_to_obj(compiler: impl AsRef<str>, include_dir: impl AsRef<Path>, file: impl AsRef<Path> + Clone, object_path: impl AsRef<Path>) -> Result<(), Error> {
        let comp_status = Command::new(compiler.as_ref())
            .arg("-fPIC")
            .arg("-c")
            .arg("-I")
            .arg(include_dir.as_ref())
            .arg("-o")
            .arg(object_path.as_ref())
            .arg(&file.as_ref().to_str().ok_or_else(|| Error::IllegalFileName(file.as_ref().to_path_buf().into_boxed_path()))?)
            .spawn()
            .map_err(Error::IO)?
            .wait()
            .map_err(Error::IO)?;
        if !comp_status.success() {
            Err(Error::NonZeroCompilerExitStatus(comp_status.code()))
        } else {
            Ok(())
        }
    }

    /// given a c++ compiler (such as g++), a c compiler (such as gcc), the root directory of a tree sitter repo, the name of
    /// the language and a working directory, compile the tree sitter parser into a dynamic library
    /// and return the path
    fn compile_dyn_lib(cpp_compiler: impl AsRef<str>, c_compiler: impl AsRef<str>, root_dir: impl AsRef<Path>, lang_name: impl AsRef<str>, target_dir: impl AsRef<Path>) -> Result<impl AsRef<Path>, Error> {
        let mut src = root_dir.as_ref().to_path_buf();
        src.push("src/");
        let cc = Libraries::files_with_ext(&src, "cc");
        let cpp = Libraries::files_with_ext(&src, "cpp");
        let c = Libraries::files_with_ext(&src, "c");
        for file in cc.into_iter().chain(cpp.into_iter()) {
            let mut object_path = target_dir.as_ref().to_path_buf();
            object_path.push(file.file_stem().expect("logic error in Libraries::files_with_ext, produced file with no prefix"));
            object_path.set_extension("o");
            Libraries::compile_to_obj(&cpp_compiler, &src, file.to_str().ok_or_else(|| Error::IllegalFileName(file.clone()))?, object_path)?;
        }
        for file in c.into_iter() {
            let mut object_path = target_dir.as_ref().to_path_buf();
            object_path.push(file.file_stem().expect("logic error in Libraries::files_with_ext, produced file with no prefix"));
            object_path.set_extension("o");
            Libraries::compile_to_obj(&c_compiler, &src, file.to_str().ok_or_else(|| Error::IllegalFileName(file.clone()))?, object_path)?;
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

        for file in Libraries::files_with_ext(target_dir, "o") {
            dyn_link_cmd.arg(file.as_os_str());
        }

        let link_status = dyn_link_cmd.spawn().map_err(Error::IO)?.wait().map_err(Error::IO)?;

        if !link_status.success() {
            return Err(Error::NonZeroCompilerExitStatus(link_status.code()));
        }

        if !dyn_lib_path.exists() {
            Err(Error::CompilerDidntProduceLib)
        } else {
            Ok(dyn_lib_path)
        }
    }

    /// given a path to a tree sitter parser dyn lib, and the language the parser is in, load and
    /// return the library
    fn load_dyn_lib(lib_path: impl AsRef<OsStr>, lang_name: impl AsRef<str>) -> Result<Library, Error> {
        let mut lang_func_name = b"tree_sitter_".to_vec();
        lang_func_name.extend(lang_name.as_ref().bytes());
        
        unsafe {Library::new(lib_path.as_ref())}.map_err(Error::Linker)
    }

    fn add_lib(&mut self, lang_name: impl AsRef<str>, target_dir: impl AsRef<Path>, root_dir: impl AsRef<Path>, cpp_compiler: impl AsRef<str>, c_compiler: impl AsRef<str>) -> Result<&Lib, Error> {
        let lib_path = Libraries::compile_dyn_lib(cpp_compiler, c_compiler, &root_dir, &lang_name, target_dir)?;
        let lib = unsafe{Library::new(lib_path.as_ref())}.map_err(Error::Linker)?;

        let mut lang_func_name = b"tree_sitter_".to_vec();
        lang_func_name.extend(lang_name.as_ref().bytes());
        
        let lang_sym: Symbol<fn() -> Language> = unsafe {lib.get(&lang_func_name)}.map_err(Error::Linker)?;
        let lang = lang_sym();

        let mut highlight_path = root_dir.as_ref().to_path_buf();
        highlight_path.push("queries/");
        highlight_path.push("highlights.scm");
        let highlighting = Query::new(lang.clone(), &fs::read_to_string(highlight_path).map_err(Error::IO)?).map_err(Error::IllegalQuery)?;

        self.libs.insert(lang_name.as_ref().to_string(), Lib {
            lib,
            lang,
            highlighting
        });
        Ok(self.libs.get(lang_name.as_ref()).unwrap())
    }

    pub fn get_or_load<'a>(&'a mut self, lang_name: impl AsRef<str>, target_dir: impl AsRef<Path>, root_dir: impl AsRef<Path>, cpp_compiler: impl AsRef<str>, c_compiler: impl AsRef<str>) -> Result<&'a Lib, Error> {
        if self.libs.contains_key(lang_name.as_ref()) {
            Ok(self.libs.get(lang_name.as_ref()).unwrap())
        } else {
            self.add_lib(lang_name, target_dir, root_dir, cpp_compiler, c_compiler)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Linker(libloading::Error),
    Compile(),
    IO(io::Error),
    IllegalFileName(Box<Path>),
    NonZeroCompilerExitStatus(Option<i32>),
    CompilerDidntProduceLib,
    IllegalQuery(QueryError)
}
