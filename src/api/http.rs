//! Active HTTP API module: result types, auth token constants, middleware,
//! handlers, router, OpenAPI, and server entry point.

pub mod auth;

pub mod handler;
pub mod middleware;
pub mod router;
pub mod server;

pub mod openapi;

pub mod result;
pub mod state;
