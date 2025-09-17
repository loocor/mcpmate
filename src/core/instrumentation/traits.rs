//! Traits for abstracting over different mutex types

use std::future::Future;

/// Trait for mutex-like types that can be locked asynchronously
pub trait AsyncMutex<T> {
    type Guard<'a>: std::ops::Deref<Target = T> + std::ops::DerefMut<Target = T> + Send + 'a
    where
        Self: 'a;

    /// Lock the mutex and return a guard
    fn lock(&self) -> impl Future<Output = Self::Guard<'_>> + Send;
}

impl<T: Send> AsyncMutex<T> for tokio::sync::Mutex<T> {
    type Guard<'a>
        = tokio::sync::MutexGuard<'a, T>
    where
        Self: 'a;

    async fn lock(&self) -> Self::Guard<'_> {
        self.lock().await
    }
}

impl<T: Send> AsyncMutex<T> for crate::core::instrumentation::InstrumentedMutex<T> {
    type Guard<'a>
        = crate::core::instrumentation::InstrumentedMutexGuard<'a, T>
    where
        Self: 'a;

    async fn lock(&self) -> Self::Guard<'_> {
        self.lock().await
    }
}
