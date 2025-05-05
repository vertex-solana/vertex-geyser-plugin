use {
    crate::config::Config,
    ::{
        agave_geyser_plugin_interface::geyser_plugin_interface::{
            GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
            ReplicaTransactionInfoVersions, Result as PluginResult,
        },
        log::info,
        std::{fmt::Debug, fs::File, io::Read},
    },
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
        solana_logger::setup_with_default("info");
        info!(
            "Loading plugin {:?} from config_file {:?}",
            self.name(),
            config_file
        );
        let mut file = File::open(config_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config: Config = serde_json::from_str(&contents).map_err(|err| {
            GeyserPluginError::ConfigFileReadError {
                msg: format!(
                    "The config file is not in the JSON format expected: {:?}",
                    err
                ),
            }
        })?;

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
        status: &agave_geyser_plugin_interface::geyser_plugin_interface::SlotStatus,
    ) -> PluginResult<()> {
        info!(
            "update_slot_status called: slot={}, parent={:?}, status={:?}",
            slot, parent, status
        );
        Ok(())
    }

    fn notify_transaction(
        &self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64,
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
