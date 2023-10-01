//! This module switches between `tokio` and `turmoil` types depending on
//! whether we are running tests or not.
#[cfg(not(feature = "turmoil"))]
pub(crate) use tokio::net::TcpStream;

#[cfg(feature = "turmoil")]
pub(crate) use turmoil::net::TcpStream;
