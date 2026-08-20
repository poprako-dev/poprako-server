use std::sync::Arc;

/// Selects one of the transaction coordinators owned by the application.
pub struct HybNucl<NR, NS>(NR, NS);

impl<NR, NS> HybNucl<NR, NS> {
    /// Combines the repeatable-read and serializable coordinators.
    pub fn new(repeatable_read: NR, serializable: NS) -> Self {
        Self(repeatable_read, serializable)
    }

    /// Returns the repeatable-read transaction coordinator.
    pub fn repeatable_read(&self) -> &NR {
        &self.0
    }

    /// Returns the serializable transaction coordinator.
    pub fn serializable(&self) -> &NS {
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
                nucl,
                repo,
                prom,
                auth,
                image_pool,
                develop,
            }),
        }
    }

    /// Returns the transaction coordinator selector.
    pub fn nucl(&self) -> &N {
        &self.inner.nucl
    }

    /// Returns the repository bundle.
    pub fn repo(&self) -> &R {
        &self.inner.repo
    }

    /// Returns the deferred-task producer.
    pub fn prom(&self) -> &P {
        &self.inner.prom
    }

    /// Returns the token authentication port.
    pub fn auth(&self) -> &A {
        &self.inner.auth
    }

    /// Returns the image pool.
    pub fn image_pool(&self) -> &I {
        &self.inner.image_pool
    }

    /// Returns the side-effect developer.
    pub fn develop(&self) -> &D {
        &self.inner.develop
    }
}
