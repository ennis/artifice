use ashley;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::fs;

const TEST_SOURCES_INCLUDE_DIR: &str = "data/tests/include";
const TEST_SOURCES_DIR: &str = "data/tests";

fn setup_include_files() -> ashley::glsl::SourceFiles {
    let mut source_files = ashley::glsl::SourceFiles::new();
    for entry in fs::read_dir(TEST_SOURCES_INCLUDE_DIR).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            if let Some(ext) = path.extension() {
                if ext == "glsl" || ext == "frag" || ext == "comp" || ext == "vert" {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    let source = fs::read_to_string(&*path).expect("could not load source file");
                    source_files.register_source(&*file_name, source);
                }
            }
        }
    }
    source_files
}

fn test_translation_file(path: &str) {
    let sources = setup_include_files();
    let mut module = ashley::ast::Module::new();
    let mut diag_writer = StandardStream::stderr(ColorChoice::Always);
    let source = fs::read_to_string(path).expect("could not load source file");
    ashley::glsl::translate_glsl(&mut module, &mut diag_writer, &sources, &source, path).unwrap();
}

macro_rules! test_glsl_source_file {
    ($name:ident) => {
        #[test]
        fn $name() {
            test_translation_file(concat!("data/tests/", stringify!($name), ".glsl"))
        }
    };
}

test_glsl_source_file!(background);
