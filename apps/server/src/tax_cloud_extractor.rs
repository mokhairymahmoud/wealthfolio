use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};
use wealthfolio_ai::{
    types::MessageAttachment, AiEnvironment, AiStreamEvent, ChatService, SendMessageRequest,
};
use wealthfolio_core::tax::{NewExtractedTaxField, TaxCloudExtractionTrait, TaxDocument};
use wealthfolio_core::{Error, Result};

pub struct AiTaxCloudExtractor<E: AiEnvironment + 'static> {
    chat_service: Arc<ChatService<E>>,
}

impl<E: AiEnvironment + 'static> AiTaxCloudExtractor<E> {
    pub fn new(chat_service: Arc<ChatService<E>>) -> Self {
        Self { chat_service }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudExtractionEnvelope {
    fields: Vec<CloudExtractedField>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudExtractedField {
    field_key: String,
    label: String,
    mapped_category: Option<String>,
    suggested_declaration_box: Option<String>,
    value_text: Option<String>,
    amount_eur: Option<Value>,
    confidence: Option<f64>,
    source_locator: Option<Value>,
}

fn extraction_prompt(filename: &str, local_text_preview: &str) -> String {
    format!(
        "You are extracting French tax declaration fields from the document '{filename}'.\n\
Return ONLY valid JSON: {{\"fields\":[{{\"fieldKey\":string,\"label\":string,\
\"mappedCategory\":string|null,\"suggestedDeclarationBox\":string|null,\
\"valueText\":string|null,\"amountEur\":number|null,\"confidence\":number,\
\"sourceLocator\":{{\"snippet\":string}}|null}}]}}.\n\
\n\
For IFU documents extract:\n\
- DIVIDENDS (box 2DC): dividendes bruts\n\
- INTEREST (box 2TR): intérêts\n\
- SECURITY_GAINS (box 2074/3VG-3VH): plus-values de cession\n\
- FEES: frais\n\
- FOREIGN_WITHHOLDING_TAX (box 2047): retenue à la source étrangère\n\
\n\
For fiche de paie / bulletin de salaire documents extract:\n\
- NET_IMPOSABLE (box 1AJ): net imposable cumulé, montant net social, totalisation brut, \
brut imposable, net à payer avant prélèvement. Use the CUMULATIVE annual figure if present, \
otherwise the monthly figure.\n\
- CSG_DEDUCTIBLE: CSG déductible cumulée\n\
- CSG_NON_DEDUCTIBLE: CSG non déductible\n\
- HEURES_SUP (box 1GH): heures supplémentaires exonérées\n\
- PRELEVEMENT_SOURCE: prélèvement à la source (montant retenu)\n\
\n\
Rules: amounts in EUR as plain numbers, confidence 0-1, omit fields not present, \
never fabricate values.\n\
\n\
Document text:\n{local_text_preview}"
    )
}

fn extract_json_payload(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(stripped) = trimmed.strip_prefix("```") {
        let stripped = stripped.trim_start();
        let stripped = stripped
            .strip_prefix("json")
            .map(str::trim_start)
            .unwrap_or(stripped);
        if let Some(end) = stripped.rfind("```") {
            return stripped[..end].trim();
        }
    }
    trimmed
}

fn parse_decimal(value: Option<Value>) -> Option<rust_decimal::Decimal> {
    match value {
        Some(Value::Number(number)) => number.to_string().parse::<rust_decimal::Decimal>().ok(),
        Some(Value::String(text)) => text.parse::<rust_decimal::Decimal>().ok(),
        _ => None,
    }
}

fn normalize_source_locator(value: Option<Value>) -> Option<String> {
    value.and_then(|value| serde_json::to_string(&value).ok())
}

async fn collect_ai_response<E: AiEnvironment + 'static>(
    chat_service: &ChatService<E>,
    request: SendMessageRequest,
) -> Result<String> {
    let thread = chat_service
        .create_thread()
        .await
        .map_err(|error| Error::Unexpected(error.to_string()))?;
    let thread_id = thread.id.clone();

    let mut stream = chat_service
        .send_message(SendMessageRequest {
            thread_id: Some(thread_id.clone()),
            ..request
        })
        .await
        .map_err(|error| Error::Unexpected(error.to_string()))?;

    let mut text = String::new();
    while let Some(event) = stream.next().await {
        match event {
            AiStreamEvent::TextDelta { delta, .. } => text.push_str(&delta),
            AiStreamEvent::Done { message, .. } => {
                let done_text = message.get_text();
                if !done_text.is_empty() {
                    text = done_text;
                }
                break;
            }
            AiStreamEvent::Error { message, .. } => {
                let _ = chat_service.delete_thread(&thread_id).await;
                return Err(Error::Unexpected(message));
            }
            _ => {}
        }
    }

    let _ = chat_service.delete_thread(&thread_id).await;

    if text.is_empty() {
        warn!("Tax cloud extraction: model returned empty response");
        return Err(Error::Unexpected(
            "AI model returned an empty response. The model may not support this request format or the content was filtered.".to_string(),
        ));
    }

    debug!(
        "Tax cloud extraction raw response: {}",
        &text[..text.len().min(500)]
    );
    Ok(text)
}

#[async_trait]
impl<E: AiEnvironment + 'static> TaxCloudExtractionTrait for AiTaxCloudExtractor<E> {
    async fn extract_tax_fields(
        &self,
        document: &TaxDocument,
        content: &[u8],
        local_text_preview: &str,
    ) -> Result<Vec<NewExtractedTaxField>> {
        let is_pdf = document
            .mime_type
            .as_deref()
            .map(|mime| mime.eq_ignore_ascii_case("application/pdf"))
            .unwrap_or_else(|| document.filename.to_ascii_lowercase().ends_with(".pdf"));

        // For PDFs with a text layer, the preview is embedded in the prompt — no attachment needed.
        // For scanned/image PDFs (empty preview), fail fast with a clear error: the PDF is
        // image-only and too large to send as base64 to any reasonable API model context window.
        // The user must provide a text-based or OCR-processed PDF.
        if is_pdf && local_text_preview.trim().is_empty() {
            return Err(Error::Unexpected(
                "This PDF appears to be a scanned image with no text layer. \
                Cloud extraction requires a text-based PDF. \
                Please use a PDF generated directly from payroll software, or \
                run OCR on the file before uploading."
                    .to_string(),
            ));
        }

        // For plain text files, send content as a text attachment.
        let attachments = if is_pdf {
            None
        } else {
            Some(vec![MessageAttachment {
                name: document.filename.clone(),
                content_type: document
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "text/plain".to_string()),
                data: String::from_utf8_lossy(content).to_string(),
            }])
        };

        let response_text = collect_ai_response(
            &self.chat_service,
            SendMessageRequest {
                content: extraction_prompt(&document.filename, local_text_preview),
                allowed_tools: Some(Vec::new()),
                attachments,
                ..Default::default()
            },
        )
        .await?;

        let payload = extract_json_payload(&response_text);
        let envelope: CloudExtractionEnvelope = serde_json::from_str(payload).map_err(|error| {
            warn!(
                "Tax cloud extraction: failed to parse response. error={error} response={}",
                &response_text[..response_text.len().min(800)]
            );
            Error::Unexpected(format!(
                "Failed to parse cloud extraction response: {error}. Raw: {}",
                &response_text[..response_text.len().min(200)]
            ))
        })?;

        Ok(envelope
            .fields
            .into_iter()
            .map(|field| NewExtractedTaxField {
                field_key: field.field_key,
                label: field.label,
                mapped_category: field.mapped_category,
                suggested_declaration_box: field.suggested_declaration_box,
                source_locator_json: normalize_source_locator(field.source_locator),
                value_text: field.value_text,
                amount_eur: parse_decimal(field.amount_eur),
                confidence: field.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
                status: "SUGGESTED".to_string(),
                confirmed_amount_eur: None,
            })
            .collect())
    }
}
