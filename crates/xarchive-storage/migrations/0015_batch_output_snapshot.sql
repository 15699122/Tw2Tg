ALTER TABLE archive_batches
    ADD COLUMN output_settings_json TEXT NOT NULL DEFAULT
    '{"naming_mode":"original","filename_template":"","export_json":true,"export_text":true}';