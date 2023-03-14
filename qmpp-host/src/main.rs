use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;

use quake_util::qmap;

use wasmtime::{Engine, Module};

mod plugin;
use plugin::{init, process};

fn main() {
    let engine = Engine::default();
    let module = Module::from_file(
        &engine,
        "target/wasm32-unknown-unknown/release/hello.wasm",
    )
    .unwrap();

    let reader = BufReader::new(
        File::open("qmpp-host/test-res/q25_limits_4lt.map").unwrap(),
    );
    let map = qmap::parse(reader).unwrap();

    init(&engine, &module);
    process(&engine, &module, Rc::new(map));
}
