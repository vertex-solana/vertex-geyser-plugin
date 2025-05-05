use {
    agave_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
    geyser_plugin_vertex::GeyserPluginVertex,
};

pub mod config;
pub mod geyser_plugin_vertex;
pub mod postgres_client;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = GeyserPluginVertex::new();
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}
