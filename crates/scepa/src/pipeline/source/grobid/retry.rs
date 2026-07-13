use std::{path::Path, time::Duration};

use rootcause::prelude::Report;
use tokio::time::sleep;

use super::{GrobidClient, GrobidError};

const MAX_ATTEMPTS: usize = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_secs(2);

impl GrobidClient {
    pub async fn extract_pdf_to_tei_xml_with_retry(
        &self,
        pdf_file: &Path,
    ) -> Result<String, Report> {
        for attempt in 1..=MAX_ATTEMPTS {
            match self.extract_pdf_to_tei_xml(pdf_file).await {
                Ok(tei_xml) => return Ok(tei_xml),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {
                    let retry_delay = retry_delay(attempt);
                    tracing::warn!(
                        pdf = %pdf_file.display(),
                        attempt,
                        max_attempts = MAX_ATTEMPTS,
                        retry_delay_secs = retry_delay.as_secs(),
                        "GROBID request failed; retrying extraction"
                    );
                    sleep(retry_delay).await;
                }
                Err(error) => {
                    return Err(error
                        .context(format!(
                            "failed to extract TEI XML from PDF `{}`",
                            pdf_file.display()
                        ))
                        .into());
                }
            }
        }

        unreachable!("extraction attempts always return or fail")
    }
}

fn retry_delay(attempt: usize) -> Duration {
    RETRY_BASE_DELAY.saturating_mul(2u32.saturating_pow((attempt - 1) as u32))
}

fn is_retryable(error: &Report) -> bool {
    error.iter_reports().any(|report| {
        report
            .downcast_current_context::<GrobidError>()
            .is_some_and(|error| matches!(error, GrobidError::Retryable(_)))
    })
}
