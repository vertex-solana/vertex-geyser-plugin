use ::{
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin,
        ReplicaAccountInfoVersions,
        ReplicaBlockInfoVersions,
        ReplicaTransactionInfoVersions,
        Result as PluginResult,
    },
    log::info,
    std::fmt::Debug,
};

#[derive(Default)]
pub struct GeyserPluginVertex {
    postgres_client: String,
}

impl Debug for GeyserPluginVertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeyserPluginVertex")
    }
}

impl GeyserPlugin for GeyserPluginVertex {
    fn name(&self) -> &'static str {
        "GeyserPluginVertex"
    }

    fn on_load(&mut self, config_file: &str, _is_reload: bool) -> PluginResult<()> {
        println!("🟢 Plugin loaded: config_file = {}", config_file);
        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Unloading plugin: {}", self.name());
    }

    fn update_account(
            &self,
            account: ReplicaAccountInfoVersions,
            slot: solana_sdk::clock::Slot,
            is_startup: bool,
        ) -> PluginResult<()> {
        Ok(())
    }

    fn notify_end_of_startup(&self) -> PluginResult<()> {
        info!("End of startup notification received");
        Ok(())
    }

    fn update_slot_status(
        &self,
        slot: u64,
        parent: Option<u64>,
        status: &agave_geyser_plugin_interface::geyser_plugin_interface::SlotStatus
    ) -> PluginResult<()> {
        info!("update_slot_status called: slot={}, parent={:?}, status={:?}", slot, parent, status);
        Ok(())
    }

    fn notify_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64
    ) -> PluginResult<()> {
        info!("notify_transaction called: slot={}", slot);
        Ok(())
    }

    fn notify_block_metadata(&self, blockinfo: ReplicaBlockInfoVersions) -> PluginResult<()> {
        info!("notify_block_metadata called");
        Ok(())
    }
}

impl GeyserPluginVertex {
    pub fn new() -> Self {
        Self::default()
    }
}
