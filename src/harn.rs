use std::sync::Arc;

/// Central application harness that wires together all port implementations.
pub struct Harn<NR, NS, R, P, A, I, D> {
    /// Shared harness storage.
    inner: Arc<HarnInner<NR, NS, R, P, A, I, D>>,
}

impl<NR, NS, R, P, A, I, D> Clone for Harn<NR, NS, R, P, A, I, D> {
    // Clones the shared harness handle.
    fn clone(&self) -> Self {
        //
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// Stores the concrete application ports.
struct HarnInner<NR, NS, R, P, A, I, D> {
    // Repeatable-read transaction coordinator.
    nucl_repeatable_read: NR,
    // Serializable transaction coordinator.
    nucl_serializable: NS,
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

impl<NR, NS, R, P, A, I, D> Harn<NR, NS, R, P, A, I, D> {
    /// Builds a harness with both supported transaction coordinators.
    pub fn new(
        nucl_repeatable_read: NR,
        nucl_serializable: NS,
        repo: R,
        prom: P,
        auth: A,
        image_pool: I,
        develop: D,
    ) -> Self {
        //
        Self {
            inner: Arc::new(HarnInner {
                nucl_repeatable_read,
                nucl_serializable,
                repo,
                prom,
                auth,
                image_pool,
                develop,
            }),
        }
    }

    /// Returns the repeatable-read transaction coordinator.
    pub fn nucl_repeatable_read(&self) -> &NR {
        &self.inner.nucl_repeatable_read
    }

    /// Returns the serializable transaction coordinator.
    pub fn nucl_serializable(&self) -> &NS {
        &self.inner.nucl_serializable
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
