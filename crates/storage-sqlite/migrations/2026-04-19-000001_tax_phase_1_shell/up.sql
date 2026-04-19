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
