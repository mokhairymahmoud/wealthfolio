use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
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
        "You are extracting French tax declaration fields from the document '{filename}'. \
Return ONLY valid JSON with this exact shape: \
{{\"fields\":[{{\"fieldKey\":\"DIVIDENDS\"|\"INTEREST\"|\"SECURITY_GAINS\"|\"FEES\"|\"NET_IMPOSABLE\"|\"CSG_DEDUCTIBLE\"|\"HEURES_SUP\",\"label\":string,\"mappedCategory\":string|null,\"suggestedDeclarationBox\":string|null,\"valueText\":string|null,\"amountEur\":number|null,\"confidence\":number,\"sourceLocator\":object|null}}]}}. \
For IFU documents extract: DIVIDENDS (box 2DC), INTEREST (box 2TR), SECURITY_GAINS (box 2074/3VG-3VH), FEES, FOREIGN_WITHHOLDING_TAX (box 2047). \
For fiche de paie documents extract: NET_IMPOSABLE (net imposable cumulé, box 1AJ), CSG_DEDUCTIBLE (CSG déductible cumulée), HEURES_SUP (heures supplémentaires exonérées, box 1GH). \
Use confidence between 0 and 1. Omit unsupported fields. Amounts must be in EUR. \
Use sourceLocator to help the user verify where the value came from. \
Here is local OCR/text extraction for reference:\n{local_text_preview}"
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
                text = message.get_text();
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
        // For PDFs, the local_text_preview already contains the extracted text and is
        // embedded in the prompt — sending the raw PDF binary as an attachment would
        // add hundreds of thousands of tokens for no benefit (non-vision models can't
        // render PDFs anyway). For plain text files, send the content as an attachment.
        let is_pdf = document
            .mime_type
            .as_deref()
            .map(|mime| mime.eq_ignore_ascii_case("application/pdf"))
            .unwrap_or_else(|| document.filename.to_ascii_lowercase().ends_with(".pdf"));

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
            Error::Unexpected(format!(
                "Failed to parse cloud extraction response: {error}"
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
