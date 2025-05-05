use ::{
    agave_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, ReplicaAccountInfoV3, ReplicaBlockInfoV3, SlotStatus,
    },
    crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender},
    log::*,
    openssl::ssl::{SslConnector, SslFiletype, SslMethod},
    postgres::{Client, NoTls, Statement},
    postgres_openssl::MakeTlsConnector,
    solana_sdk::timing::AtomicInterval,
    std::{
        collections::HashSet,
        sync::{
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread::{self, sleep, Builder, JoinHandle},
        time::Duration,
    },
};

use crate::config::{GeyserPluginPostgresConfig, GeyserPluginPostgresError};

/// The maximum asynchronous requests allowed in the channel to avoid excessive
/// memory usage. The downside -- calls after this threshold is reached can get blocked.
const MAX_ASYNC_REQUESTS: usize = 40960;
const SAFE_BATCH_STARTING_SLOT_CUSHION: u64 = 2 * 40960;
const DEFAULT_POSTGRES_PORT: u16 = 5432;
const DEFAULT_THREADS_COUNT: usize = 100;
const DEFAULT_ACCOUNTS_INSERT_BATCH_SIZE: usize = 10;
const ACCOUNT_COLUMN_COUNT: usize = 10;
const DEFAULT_PANIC_ON_DB_ERROR: bool = false;
const DEFAULT_STORE_ACCOUNT_HISTORICAL_DATA: bool = false;

struct PostgresSqlClientWrapper {
    client: Client,
    // update_account_stmt: Statement,
    // bulk_account_insert_stmt: Statement,
    // update_slot_with_parent_stmt: Statement,
    // update_slot_without_parent_stmt: Statement,
    // insert_account_audit_stmt: Option<Statement>,
}

pub struct SimplePostgresClient {
    batch_size: usize,
    slots_at_startup: HashSet<u64>,
    pending_account_updates: Vec<DbAccountInfo>,
    index_token_owner: bool,
    index_token_mint: bool,
    client: Mutex<PostgresSqlClientWrapper>,
}

struct PostgresClientWorker {
    client: SimplePostgresClient,
    /// Indicating if accounts notification during startup is done.
    is_startup_done: bool,
}

impl PostgresClientWorker {
    fn new(config: GeyserPluginPostgresConfig) -> Result<Self, GeyserPluginError> {
        let result = SimplePostgresClient::new(&config);
        match result {
            Ok(client) => Ok(PostgresClientWorker {
                client,
                is_startup_done: false,
            }),
            Err(err) => {
                error!("Error in creating SimplePostgresClient: {}", err);
                Err(err)
            }
        }
    }

    fn do_work(
        &mut self,
        receiver: Receiver<DbWorkItem>,
        exit_worker: Arc<AtomicBool>,
        is_startup_done: Arc<AtomicBool>,
        startup_done_count: Arc<AtomicUsize>,
        panic_on_db_errors: bool,
    ) -> Result<(), GeyserPluginError> {
        Ok(())
    }
}

impl SimplePostgresClient {
    pub fn connect_to_db(config: &GeyserPluginPostgresConfig) -> Result<Client, GeyserPluginError> {
        let port = config.port.unwrap_or(DEFAULT_POSTGRES_PORT);

        let connection_str = if let Some(connection_str) = &config.connection_str {
            connection_str.clone()
        } else {
            if config.host.is_none() || config.user.is_none() {
                let msg = format!(
                    "\"connection_str\": {:?}, or \"host\": {:?} \"user\": {:?} must be specified",
                    config.connection_str, config.host, config.user
                );
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            format!(
                "host={} user={} port={}",
                config.host.as_ref().unwrap(),
                config.user.as_ref().unwrap(),
                port
            )
        };

        let result = if let Some(true) = config.use_ssl {
            if config.server_ca.is_none() {
                let msg = "\"server_ca\" must be specified when \"use_ssl\" is set".to_string();
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            if config.client_cert.is_none() {
                let msg = "\"client_cert\" must be specified when \"use_ssl\" is set".to_string();
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            if config.client_key.is_none() {
                let msg = "\"client_key\" must be specified when \"use_ssl\" is set".to_string();
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
            if let Err(err) = builder.set_ca_file(config.server_ca.as_ref().unwrap()) {
                let msg = format!(
                    "Failed to set the server certificate specified by \"server_ca\": {}. Error: ({})",
                    config.server_ca.as_ref().unwrap(),
                    err
                );
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            if let Err(err) =
                builder.set_certificate_file(config.client_cert.as_ref().unwrap(), SslFiletype::PEM)
            {
                let msg = format!(
                    "Failed to set the client certificate specified by \"client_cert\": {}. Error: ({})",
                    config.client_cert.as_ref().unwrap(),
                    err
                );
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }
            if let Err(err) =
                builder.set_private_key_file(config.client_key.as_ref().unwrap(), SslFiletype::PEM)
            {
                let msg = format!(
                    "Failed to set the client key specified by \"client_key\": {}. Error: ({})",
                    config.client_key.as_ref().unwrap(),
                    err
                );
                return Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::ConfigurationError { msg },
                )));
            }

            let mut connector = MakeTlsConnector::new(builder.build());
            connector.set_callback(|connect_config, _domain| {
                connect_config.set_verify_hostname(false);
                Ok(())
            });
            Client::connect(&connection_str, connector)
        } else {
            Client::connect(&connection_str, NoTls)
        };

        match result {
            Err(err) => {
                let msg = format!(
                    "Error in connecting to the PostgreSQL database: {:?} connection_str: {:?}",
                    err, connection_str
                );
                error!("{}", msg);
                Err(GeyserPluginError::Custom(Box::new(
                    GeyserPluginPostgresError::DataStoreConnectionError { msg },
                )))
            }
            Ok(client) => Ok(client),
        }
    }

    pub fn new(config: &GeyserPluginPostgresConfig) -> Result<Self, GeyserPluginError> {
        info!("Creating SimplePostgresClient...");
        let mut client = Self::connect_to_db(config)?;

        let batch_size = config
            .batch_size
            .unwrap_or(DEFAULT_ACCOUNTS_INSERT_BATCH_SIZE);

        let store_account_historical_data = config
            .store_account_historical_data
            .unwrap_or(DEFAULT_STORE_ACCOUNT_HISTORICAL_DATA);

        info!("Created SimplePostgresClient.");
        Ok(Self {
            batch_size,
            pending_account_updates: Vec::with_capacity(batch_size),
            client: Mutex::new(PostgresSqlClientWrapper { client }),
            index_token_owner: config.index_token_owner.unwrap_or_default(),
            index_token_mint: config.index_token_mint.unwrap_or(false),
            slots_at_startup: HashSet::default(),
        })
    }

    pub fn get_highest_available_slot(&mut self) -> Result<u64, GeyserPluginError> {
        let client = self.client.get_mut().unwrap();

        let last_slot_query = "SELECT slot FROM slot ORDER BY slot DESC LIMIT 1;";

        let result = client.client.query_opt(last_slot_query, &[]);
        match result {
            Ok(opt_slot) => Ok(opt_slot
                .map(|row| {
                    let raw_slot: i64 = row.get(0);
                    raw_slot as u64
                })
                .unwrap_or(0)),
            Err(err) => {
                let msg = format!(
                    "Failed to receive last slot from PostgreSQL database. Error: {:?}",
                    err
                );
                error!("{}", msg);
                Err(GeyserPluginError::AccountsUpdateError { msg })
            }
        }
    }
}

impl Eq for DbAccountInfo {}

#[derive(Clone, PartialEq, Debug)]
pub struct DbAccountInfo {
    pub pubkey: Vec<u8>,
    pub lamports: i64,
    pub owner: Vec<u8>,
    pub executable: bool,
    pub rent_epoch: i64,
    pub data: Vec<u8>,
    pub slot: i64,
    pub write_version: i64,
    pub txn_signature: Option<Vec<u8>>,
}

pub(crate) fn abort() -> ! {
    #[cfg(not(test))]
    {
        // standard error is usually redirected to a log file, cry for help on standard output as
        // well
        eprintln!("Validator process aborted. The validator log may contain further details");
        std::process::exit(1);
    }

    #[cfg(test)]
    panic!("process::exit(1) is intercepted for friendly test failure...");
}

struct UpdateAccountRequest {
    account: DbAccountInfo,
    is_startup: bool,
}

struct UpdateSlotRequest {
    slot: u64,
    parent: Option<u64>,
    slot_status: SlotStatus,
}

#[warn(clippy::large_enum_variant)]
enum DbWorkItem {
    UpdateAccount(Box<UpdateAccountRequest>),
    UpdateSlot(Box<UpdateSlotRequest>),
}

pub struct ParallelPostgresClient {
    workers: Vec<JoinHandle<Result<(), GeyserPluginError>>>,
    exit_worker: Arc<AtomicBool>,
    is_startup_done: Arc<AtomicBool>,
    startup_done_count: Arc<AtomicUsize>,
    initialized_worker_count: Arc<AtomicUsize>,
    sender: Sender<DbWorkItem>,
    last_report: AtomicInterval,
    transaction_write_version: AtomicU64,
}

impl ParallelPostgresClient {
    pub fn new(config: &GeyserPluginPostgresConfig) -> Result<Self, GeyserPluginError> {
        info!("Creating ParallelPostgresClient...");
        let (sender, receiver) = bounded(MAX_ASYNC_REQUESTS);
        let exit_worker = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::default();
        let is_startup_done = Arc::new(AtomicBool::new(false));
        let startup_done_count = Arc::new(AtomicUsize::new(0));
        let worker_count = config.threads.unwrap_or(DEFAULT_THREADS_COUNT);
        let initialized_worker_count = Arc::new(AtomicUsize::new(0));
        for i in 0..worker_count {
            let cloned_receiver = receiver.clone();
            let exit_clone = exit_worker.clone();
            let is_startup_done_clone = is_startup_done.clone();
            let startup_done_count_clone = startup_done_count.clone();
            let initialized_worker_count_clone = initialized_worker_count.clone();
            let config = config.clone();
            let worker = Builder::new()
                .name(format!("worker-{}", i))
                .spawn(move || -> Result<(), GeyserPluginError> {
                    let panic_on_db_errors = *config
                        .panic_on_db_errors
                        .as_ref()
                        .unwrap_or(&DEFAULT_PANIC_ON_DB_ERROR);
                    let result = PostgresClientWorker::new(config);

                    match result {
                        Ok(mut worker) => {
                            initialized_worker_count_clone.fetch_add(1, Ordering::Relaxed);
                            worker.do_work(
                                cloned_receiver,
                                exit_clone,
                                is_startup_done_clone,
                                startup_done_count_clone,
                                panic_on_db_errors,
                            )?;
                            Ok(())
                        }
                        Err(err) => {
                            error!("Error when making connection to database: ({})", err);
                            if panic_on_db_errors {
                                abort();
                            }
                            Err(err)
                        }
                    }
                })
                .unwrap();

            workers.push(worker);
        }

        info!("Created ParallelPostgresClient.");
        Ok(Self {
            last_report: AtomicInterval::default(),
            workers,
            exit_worker,
            is_startup_done,
            startup_done_count,
            initialized_worker_count,
            sender,
            transaction_write_version: AtomicU64::default(),
        })
    }
}

pub struct PostgresClientBuilder {}

impl PostgresClientBuilder {
    pub fn build_pararallel_postgres_client(
        config: &GeyserPluginPostgresConfig,
    ) -> Result<(ParallelPostgresClient, Option<u64>), GeyserPluginError> {
        let batch_optimize_by_skiping_older_slots =
            match config.skip_upsert_existing_accounts_at_startup {
                true => {
                    let mut on_load_client = SimplePostgresClient::new(config)?;

                    // database if populated concurrently so we need to move some number of slots
                    // below highest available slot to make sure we do not skip anything that was already in DB.
                    let batch_slot_bound = on_load_client
                        .get_highest_available_slot()?
                        .saturating_sub(SAFE_BATCH_STARTING_SLOT_CUSHION);
                    info!(
                        "Set batch_optimize_by_skiping_older_slots to {}",
                        batch_slot_bound
                    );
                    Some(batch_slot_bound)
                }
                false => None,
            };

        ParallelPostgresClient::new(config).map(|v| (v, batch_optimize_by_skiping_older_slots))
    }
}
