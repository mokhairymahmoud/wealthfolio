ALTER TABLE extracted_tax_fields
    ADD COLUMN suggested_declaration_box TEXT;

ALTER TABLE extracted_tax_fields
    ADD COLUMN source_locator_json TEXT;
