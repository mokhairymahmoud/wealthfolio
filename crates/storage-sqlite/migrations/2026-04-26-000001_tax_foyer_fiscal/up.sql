ALTER TABLE tax_profiles ADD COLUMN situation_familiale TEXT NOT NULL DEFAULT 'CELIBATAIRE';
ALTER TABLE tax_profiles ADD COLUMN nombre_enfants INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tax_profiles ADD COLUMN nombre_enfants_handicapes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tax_profiles ADD COLUMN parent_isole INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tax_profiles ADD COLUMN ancien_combattant_ou_invalidite INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tax_profiles ADD COLUMN nombre_parts REAL NOT NULL DEFAULT 1.0;
