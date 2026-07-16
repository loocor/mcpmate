use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
#[error("All eligible upstream servers failed to list {capability}: {failures}")]
pub(crate) struct AggregateListError {
    capability: &'static str,
    failures: String,
}

#[derive(Debug, thiserror::Error)]
#[error("The upstream {capability} listing is incomplete: {failures}")]
pub(crate) struct AggregateListIncompleteError {
    capability: &'static str,
    failures: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AggregateListCompletionError {
    #[error(transparent)]
    AllFailed(#[from] AggregateListError),
    #[error(transparent)]
    Incomplete(#[from] AggregateListIncompleteError),
}

pub(crate) struct AggregateListStatus {
    capability: &'static str,
    attempted: usize,
    failures: Vec<String>,
}

impl AggregateListStatus {
    pub(crate) fn new(capability: &'static str) -> Self {
        Self {
            capability,
            attempted: 0,
            failures: Vec::new(),
        }
    }

    pub(crate) fn record_success(&mut self) {
        self.attempted += 1;
    }

    pub(crate) fn record_failure(
        &mut self,
        server_id: &str,
        server_name: &str,
        error: impl Display,
    ) {
        self.attempted += 1;
        let failure = format!("{server_name} ({server_id}): {error}");
        tracing::warn!(
            capability = self.capability,
            server_id,
            server_name,
            error = %error,
            "Skipping failed upstream capability listing"
        );
        self.failures.push(failure);
    }

    pub(crate) fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    pub(crate) fn failure_summary(&self) -> Option<String> {
        self.has_failures()
            .then(|| format!("{}: {}", self.capability, self.failures.join("; ")))
    }

    pub(crate) fn finish(&self) -> Result<(), AggregateListError> {
        if self.attempted > 0 && self.attempted == self.failures.len() {
            return Err(AggregateListError {
                capability: self.capability,
                failures: self.failures.join("; "),
            });
        }
        Ok(())
    }

    pub(crate) fn ensure_complete(&self) -> Result<(), AggregateListIncompleteError> {
        if self.has_failures() {
            return Err(AggregateListIncompleteError {
                capability: self.capability,
                failures: self.failures.join("; "),
            });
        }
        Ok(())
    }

    pub(crate) fn finish_for_result(
        &self,
        has_usable_entries: bool,
    ) -> Result<(), AggregateListCompletionError> {
        self.finish()?;
        if !has_usable_entries {
            self.ensure_complete()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_partial_success() {
        let mut status = AggregateListStatus::new("tools");
        status.record_failure("server-a", "alpha", "offline");
        status.record_success();

        assert!(status.finish().is_ok());
    }

    #[test]
    fn fails_when_every_upstream_fails() {
        let mut status = AggregateListStatus::new("tools");
        status.record_failure("server-a", "alpha", "offline");
        status.record_failure("server-b", "beta", "timeout");

        assert!(status.finish().is_err());
    }

    #[test]
    fn allows_empty_aggregate() {
        assert!(AggregateListStatus::new("tools").finish().is_ok());
    }

    #[test]
    fn reports_partial_listing_as_incomplete_for_authoritative_lookup() {
        let mut status = AggregateListStatus::new("tools");
        status.record_failure("server-a", "alpha", "offline");
        status.record_success();

        assert!(status.finish().is_ok());
        assert!(status.ensure_complete().is_err());
    }

    #[test]
    fn rejects_empty_results_from_a_partial_listing() {
        let mut status = AggregateListStatus::new("tools");
        status.record_failure("server-a", "alpha", "offline");
        status.record_success();

        assert!(status.finish_for_result(false).is_err());
        assert!(status.finish_for_result(true).is_ok());
    }
}
