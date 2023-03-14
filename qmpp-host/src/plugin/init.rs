use wasmtime::{Caller, Engine, Linker, Module, Store};

use super::common::{log_error, log_info, recv_bytes, PluginEnv};

#[derive(Clone)]
struct InitEnv {
    plugin_name: String,
}

impl PluginEnv for InitEnv {
    fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
}

pub fn init(engine: &Engine, module: &Module) {
    let init_env = InitEnv {
        plugin_name: String::from("hello"),
    };

    let mut store = Store::new(engine, init_env);
    let mut linker = Linker::new(engine);

    linker.func_wrap("env", "QMPP_register", register).unwrap();

    linker.func_wrap("env", "QMPP_log_info", log_info).unwrap();

    linker
        .func_wrap("env", "QMPP_log_error", log_error)
        .unwrap();

    stub_func!(linker, "env", "init", "QMPP_entity_exists", i32, i32,).unwrap();

    stub_func!(linker, "env", "init", "QMPP_brush_exists", (i32, i32), i32,)
        .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_surface_exists",
        (i32, i32, i32),
        i32
    )
    .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_keyvalue_init_read",
        (i32, i32, i32),
        i32,
    )
    .unwrap();

    stub_func!(linker, "env", "init", "QMPP_keyvalue_read", i32, (),).unwrap();

    stub_func!(linker, "env", "init", "QMPP_keys_init_read", i32, i32,)
        .unwrap();

    stub_func!(linker, "env", "init", "QMPP_keys_read", i32, (),).unwrap();

    stub_func!(linker, "env", "init", "QMPP_ehandle_count", (), i32,).unwrap();

    stub_func!(linker, "env", "init", "QMPP_bhandle_count", i32, i32,).unwrap();

    stub_func!(linker, "env", "init", "QMPP_shandle_count", (i32, i32), i32,)
        .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_texture_init_read",
        (i32, i32, i32),
        i32,
    )
    .unwrap();

    stub_func!(linker, "env", "init", "QMPP_texture_read", i32, (),).unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_half_space_read",
        (i32, i32, i32, i32),
        (),
    )
    .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_texture_alignment_read",
        (i32, i32, i32, i32),
        (),
    )
    .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_texture_alignment_is_valve",
        (i32, i32, i32),
        i32,
    )
    .unwrap();

    stub_func!(
        linker,
        "env",
        "init",
        "QMPP_texture_axes_read",
        (i32, i32, i32, i32),
        (),
    )
    .unwrap();

    let instance = linker.instantiate(&mut store, module).unwrap();

    let init_func = instance.get_func(&mut store, "QMPP_Hook_init").unwrap();
    init_func.call(&mut store, &[], &mut []).unwrap();
}

fn register(
    mut caller: Caller<'_, InitEnv>,
    name_len: i32,
    name_ptr: i32,
) -> anyhow::Result<()> {
    match recv_bytes(&mut caller, name_len, name_ptr) {
        Result::Ok(bytes) => match String::from_utf8(bytes) {
            Result::Ok(plugin_name) => {
                println!("Registered plugin '{}'", plugin_name,);
                Ok(())
            }
            Result::Err(_) => {
                Err(anyhow::anyhow!("Invalid UTF-8 in plugin name"))
            }
        },
        Result::Err(_) => Err(anyhow::anyhow!("Invalid UTF-8 in plugin name")),
    }
}
