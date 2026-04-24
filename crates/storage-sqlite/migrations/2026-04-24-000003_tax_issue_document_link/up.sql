ALTER TABLE tax_issues
    ADD COLUMN document_id TEXT REFERENCES tax_documents (id)
    ON DELETE SET NULL ON UPDATE CASCADE;
