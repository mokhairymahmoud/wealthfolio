CREATE TABLE tax_profiles (
    id TEXT NOT NULL PRIMARY KEY,
    jurisdiction TEXT NOT NULL,
    tax_residence_country TEXT NOT NULL,
    default_tax_regime TEXT NOT NULL,
    pfu_or_bareme_preference TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE account_tax_profiles (
    account_id TEXT NOT NULL PRIMARY KEY,
    jurisdiction TEXT NOT NULL,
    regime TEXT NOT NULL,
    opened_on TEXT,
    closed_on TEXT,
    metadata TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT account_tax_profiles_account_id_fkey
        FOREIGN KEY (account_id) REFERENCES accounts (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE tax_year_reports (
    id TEXT NOT NULL PRIMARY KEY,
    tax_year INTEGER NOT NULL,
    jurisdiction TEXT NOT NULL,
    status TEXT NOT NULL,
    rule_pack_version TEXT NOT NULL,
    base_currency TEXT NOT NULL,
    generated_at DATETIME,
    finalized_at DATETIME,
    assumptions_json TEXT NOT NULL DEFAULT '{}',
    summary_json TEXT NOT NULL DEFAULT '{}',
    parent_report_id TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_year_reports_parent_report_id_fkey
        FOREIGN KEY (parent_report_id) REFERENCES tax_year_reports (id)
        ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE UNIQUE INDEX tax_year_reports_one_draft_per_year
    ON tax_year_reports (tax_year, jurisdiction, status)
    WHERE status = 'DRAFT';

CREATE INDEX tax_year_reports_tax_year_idx
    ON tax_year_reports (tax_year, jurisdiction);

CREATE TABLE tax_events (
    id TEXT NOT NULL PRIMARY KEY,
    report_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    category TEXT NOT NULL,
    suggested_box TEXT,
    account_id TEXT NOT NULL,
    asset_id TEXT,
    activity_id TEXT,
    event_date TEXT NOT NULL,
    amount_currency TEXT NOT NULL,
    amount_local TEXT,
    amount_eur TEXT,
    taxable_amount_eur TEXT,
    expenses_eur TEXT,
    confidence TEXT NOT NULL,
    included INTEGER NOT NULL DEFAULT 1,
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_events_report_id_fkey
        FOREIGN KEY (report_id) REFERENCES tax_year_reports (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX tax_events_report_id_idx
    ON tax_events (report_id, event_type, event_date);

CREATE TABLE tax_event_sources (
    id TEXT NOT NULL PRIMARY KEY,
    tax_event_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_event_sources_tax_event_id_fkey
        FOREIGN KEY (tax_event_id) REFERENCES tax_events (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE tax_lot_allocations (
    id TEXT NOT NULL PRIMARY KEY,
    tax_event_id TEXT NOT NULL,
    source_activity_id TEXT NOT NULL,
    quantity TEXT NOT NULL,
    acquisition_date TEXT NOT NULL,
    cost_basis_eur TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_lot_allocations_tax_event_id_fkey
        FOREIGN KEY (tax_event_id) REFERENCES tax_events (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE tax_issues (
    id TEXT NOT NULL PRIMARY KEY,
    report_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    code TEXT NOT NULL,
    message TEXT NOT NULL,
    account_id TEXT,
    activity_id TEXT,
    tax_event_id TEXT,
    resolved_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_issues_report_id_fkey
        FOREIGN KEY (report_id) REFERENCES tax_year_reports (id)
        ON DELETE CASCADE ON UPDATE CASCADE,
    CONSTRAINT tax_issues_tax_event_id_fkey
        FOREIGN KEY (tax_event_id) REFERENCES tax_events (id)
        ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE INDEX tax_issues_report_id_idx
    ON tax_issues (report_id, severity, code);

CREATE TABLE tax_documents (
    id TEXT NOT NULL PRIMARY KEY,
    report_id TEXT NOT NULL,
    document_type TEXT NOT NULL,
    filename TEXT NOT NULL,
    mime_type TEXT,
    sha256 TEXT NOT NULL,
    encrypted_content TEXT NOT NULL,
    encryption_key_ref TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    uploaded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_documents_report_id_fkey
        FOREIGN KEY (report_id) REFERENCES tax_year_reports (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX tax_documents_report_id_idx
    ON tax_documents (report_id, document_type);

CREATE TABLE tax_document_extractions (
    id TEXT NOT NULL PRIMARY KEY,
    document_id TEXT NOT NULL,
    method TEXT NOT NULL,
    status TEXT NOT NULL,
    consent_granted INTEGER NOT NULL DEFAULT 0,
    raw_text_preview TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_document_extractions_document_id_fkey
        FOREIGN KEY (document_id) REFERENCES tax_documents (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE extracted_tax_fields (
    id TEXT NOT NULL PRIMARY KEY,
    extraction_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    label TEXT NOT NULL,
    mapped_category TEXT,
    value_text TEXT,
    amount_eur TEXT,
    confidence REAL NOT NULL,
    status TEXT NOT NULL,
    confirmed_amount_eur TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT extracted_tax_fields_extraction_id_fkey
        FOREIGN KEY (extraction_id) REFERENCES tax_document_extractions (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX extracted_tax_fields_extraction_id_idx
    ON extracted_tax_fields (extraction_id, mapped_category);

CREATE TABLE tax_reconciliation_entries (
    id TEXT NOT NULL PRIMARY KEY,
    report_id TEXT NOT NULL,
    category TEXT NOT NULL,
    suggested_box TEXT,
    app_amount_eur TEXT,
    document_amount_eur TEXT,
    selected_amount_eur TEXT,
    delta_eur TEXT,
    status TEXT NOT NULL,
    notes TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT tax_reconciliation_entries_report_id_fkey
        FOREIGN KEY (report_id) REFERENCES tax_year_reports (id)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE UNIQUE INDEX tax_reconciliation_entries_report_category_idx
    ON tax_reconciliation_entries (report_id, category);
