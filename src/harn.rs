use std::sync::Arc;

use crate::config::AppConfig;

/// Selects one of the transaction coordinators owned by the application.
pub struct HybNucl<NR, NS>(NR, NS);

impl<NR, NS> HybNucl<NR, NS> {
    /// Combines the repeatable-read and serializable coordinators.
    pub const fn new(rept_read: NR, serial: NS) -> Self {
        Self(rept_read, serial)
    }

    /// Returns the repeatable-read transaction coordinator.
    pub const fn rept_read(&self) -> &NR {
        &self.0
    }

    /// Returns the serializable transaction coordinator.
    pub const fn serial(&self) -> &NS {
        &self.1
    }
}

/// Central application harness that wires together all port implementations.
pub struct Harn<N, R, P, A, I, D> {
    /// Shared harness storage.
    inner: Arc<HarnInner<N, R, P, A, I, D>>,
}

impl<N, R, P, A, I, D> Clone for Harn<N, R, P, A, I, D> {
    // Clones the shared harness handle.
    fn clone(&self) -> Self {
        //
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Stores the concrete application ports.
struct HarnInner<N, R, P, A, I, D> {
    //
    // Runtime application configuration.
    config: AppConfig,
    // Transaction coordinator selector.
    nucl: N,
    // Repository adapter bundle.
    repo: R,
    // Deferred-task producer.
    prom: P,
    // Authentication adapter.
    auth: A,
    // Image storage adapter.
    image_pool: I,
    // Side-effect dispatcher.
    develop: D,
}

impl<N, R, P, A, I, D> Harn<N, R, P, A, I, D> {
    /// Builds a harness from its application ports.
    pub fn new(
        config: AppConfig,
        nucl: N,
        repo: R,
        prom: P,
        auth: A,
        image_pool: I,
        develop: D,
    ) -> Self {
        //
        Self {
            inner: Arc::new(HarnInner {
                config,
                nucl,
                repo,
                prom,
                auth,
                image_pool,
                develop,
            }),
        }
    }

    /// Returns the runtime application configuration.
    #[must_use]
    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    /// Returns the transaction coordinator selector.
    #[must_use]
    pub fn nucl(&self) -> &N {
        &self.inner.nucl
    }

    /// Returns the repository bundle.
    #[must_use]
    pub fn repo(&self) -> &R {
        &self.inner.repo
    }

    /// Returns the deferred-task producer.
    #[must_use]
    pub fn prom(&self) -> &P {
        &self.inner.prom
    }

    /// Returns the token authentication port.
    #[must_use]
    pub fn auth(&self) -> &A {
        &self.inner.auth
    }

    /// Returns the image pool.
    #[must_use]
    pub fn image_pool(&self) -> &I {
        &self.inner.image_pool
    }

    /// Returns the side-effect developer.
    #[must_use]
    pub fn develop(&self) -> &D {
        &self.inner.develop
    }
}
