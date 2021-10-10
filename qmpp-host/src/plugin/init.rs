use wasmer::{
    imports, Function, Instance, LazyInit, Memory, Module, WasmerEnv,
};

use crate::stub_import;
use super::common::{log_error, log_info, recv_bytes, PluginEnv};

#[derive(WasmerEnv, Clone)]
struct InitEnv {
    plugin_name: String,

    #[wasmer(export)]
    memory: LazyInit<Memory>,
}

impl PluginEnv for InitEnv {
    fn memory(&self) -> &Memory {
        self.memory.get_ref().unwrap()
    }

    fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
}

pub fn init(module: &Module) {
    let init_env = InitEnv {
        plugin_name: String::from("hello"),
        memory: LazyInit::new(),
    };

    let import_object = imports! {
        "env" => {
            "QMPP_register" => Function::new_native_with_env(
                module.store(),
                init_env.clone(),
                register
            ),
            "QMPP_log_info" => Function::new_native_with_env(
                module.store(),
                init_env.clone(),
                log_info
            ),
            "QMPP_log_error" => Function::new_native_with_env(
                module.store(),
                init_env,
                log_error
            ),

            "QMPP_entity_count" => Function::new_native(
                module.store(),
                stub_import!("QMPP_entity_count", "init", (), u32)
            ),
            "QMPP_keyvalue_init_read" => Function::new_native(
                module.store(),
                stub_import!(
                    "QMPP_keyvalue_init_read",
                     "init",
                     (u32, u32, u32),
                     u32,
                )
            ),
            "QMPP_keyvalue_read" => Function::new_native(
                module.store(),
                stub_import!(
                    "QMPP_keyvalue_read",
                    "init",
                    u32,
                    u32,
                )
            ),
        }
    };

    let instance = Instance::new(module, &import_object).unwrap();

    let init_export = instance.exports.get_function("QMPP_Hook_init").unwrap();
    init_export.call(&[]).unwrap();
}

fn register(env: &InitEnv, name_len: u32, name_ptr: u32) {
    match recv_bytes(env.memory.get_ref().unwrap(), name_len, name_ptr) {
        Result::Ok(bytes) => match String::from_utf8(bytes) {
            Result::Ok(plugin_name) => {
                println!("Registered plugin '{}'", plugin_name,)
            }
            Result::Err(_) => eprintln!("Invalid UTF-8 in plugin name"),
        },
        Result::Err(_) => eprintln!("Error while receiving bytes"),
    }
}
