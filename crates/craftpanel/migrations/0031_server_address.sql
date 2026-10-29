ALTER TABLE servers ADD COLUMN public_address TEXT
    CHECK (public_address IS NULL OR length(public_address) BETWEEN 1 AND 253);
