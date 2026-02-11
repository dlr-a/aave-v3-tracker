use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrackerError {
    #[error("Database Error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection Pool Error: {0}")]
    Pool(#[from] diesel_async::pooled_connection::deadpool::PoolError),

    #[error("Contract Interaction Error: {0}")]
    Contract(#[from] alloy::contract::Error),

    #[error("RPC Transport Error: {0}")]
    Transport(#[from] alloy::providers::transport::TransportError),
}
