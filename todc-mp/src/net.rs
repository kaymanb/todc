//! This module abstracts over `tokio` and `turmoil` depending on whether
//! we are running tests or not.

#[cfg(not(feature = "turmoil"))]
pub(crate) use tokio::net::TcpStream;

#[cfg(feature = "turmoil")]
pub(crate) use turmoil::net::TcpStream;
