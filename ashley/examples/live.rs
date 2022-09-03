use anyhow::Result;
use graal_fx::{Arena, Module};
use logos::{Lexer, Logos};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{fs::File, io::Read, thread};

const TESTBENCH_PATH: &str = "graal-fx/tests/testbench.gfx";

fn reparse() -> Result<()> {
    let mut contents = String::new();
    let mut file = File::open(TESTBENCH_PATH)?;
    file.read_to_string(&mut contents)?;
    // run the lexer
    eprintln!("--- Parser ---");
    let mut arena = Arena::new();
    let module = Module::parse(&contents, &arena);
    match module {
        Ok(m) => eprintln!("{:#?}", m),
        Err(e) => eprintln!("{:?}", e),
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(|res| {
        match res {
            Ok(event) => {
                //println!("watch event: {:?}", event);
                if let Err(e) = reparse() {
                    println!("parse error: {}", e)
                }
            }
            Err(e) => println!("watch error: {}", e),
        }
    })?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(TESTBENCH_PATH, RecursiveMode::NonRecursive)?;

    thread::park();
    Ok(())
}
