use ::{
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, Result as PluginResult,
    },
    serde_derive::{Deserialize, Serialize},
    serde_json,
    std::{fs::read_to_string, path::Path},
    thiserror::Error,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub postgres: GeyserPluginPostgresConfig,
}

impl Config {
    fn load_from_str(config: &str) -> PluginResult<Self> {
        serde_json::from_str(config).map_err(|error| GeyserPluginError::ConfigFileReadError {
            msg: error.to_string(),
        })
    }

    pub fn load_from_file<P: AsRef<Path>>(file: P) -> PluginResult<Self> {
        let config = read_to_string(file).map_err(GeyserPluginError::ConfigFileOpenError)?;
        Self::load_from_str(&config)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeyserPluginPostgresConfig {
    /// The host name or IP of the PostgreSQL server
    pub host: Option<String>,

    /// The user name of the PostgreSQL server.
    pub user: Option<String>,

    /// The port number of the PostgreSQL database, the default is 5432
    pub port: Option<u16>,

    /// The connection string of PostgreSQL database, if this is set
    /// `host`, `user` and `port` will be ignored.
    pub connection_str: Option<String>,

    /// Controls the number of threads establishing connections to
    /// the PostgreSQL server. The default is 10.
    pub threads: Option<usize>,

    /// Controls the batch size when bulk loading accounts.
    /// The default is 10.
    pub batch_size: Option<usize>,

    /// Controls whether to panic the validator in case of errors
    /// writing to PostgreSQL server. The default is false
    pub panic_on_db_errors: Option<bool>,

    /// Indicates whether to store historical data for accounts
    pub store_account_historical_data: Option<bool>,

    /// Controls whether to use SSL based connection to the database server.
    /// The default is false
    pub use_ssl: Option<bool>,

    /// Specify the path to PostgreSQL server's certificate file
    pub server_ca: Option<String>,

    /// Specify the path to the local client's certificate file
    pub client_cert: Option<String>,

    /// Specify the path to the local client's private PEM key file.
    pub client_key: Option<String>,

    /// Controls whether to index the token owners. The default is false
    pub index_token_owner: Option<bool>,

    /// Controls whetherf to index the token mints. The default is false
    pub index_token_mint: Option<bool>,

    /// Controls if this plugin can read the database on_load() to find heighest slot
    /// and ignore upsetr accounts (at_startup) that should already exist in DB
    #[serde(default)]
    pub skip_upsert_existing_accounts_at_startup: bool,
}

#[derive(Error, Debug)]
pub enum GeyserPluginPostgresError {
    #[error("Error connecting to the backend data store. Error message: ({msg})")]
    DataStoreConnectionError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    DataSchemaError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    ConfigurationError { msg: String },

    #[error("Replica account V0.0.1 not supported anymore")]
    ReplicaAccountV001NotSupported,
}
