use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use quake_util::qmap;

use wasmer::{Module, Store};

mod plugin;
use plugin::{init, process};

fn main() {
    let store = Store::default();
    let module = Module::from_file(
        &store,
        "target/wasm32-unknown-unknown/release/hello.wasm",
    )
    .unwrap();

    let reader = BufReader::new(
        File::open("qmpp-host/test-res/q25_limits_4lt.map").unwrap(),
    );
    let map = qmap::parse(reader).unwrap();

    init(&module);
    process(&module, Arc::new(map));
}
