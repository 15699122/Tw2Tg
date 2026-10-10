ALTER TABLE archive_batches
    ADD COLUMN downloader_arguments_json TEXT NOT NULL DEFAULT
    '{"gallery_dl":[],"aria2":[],"policy_version":1,"tool_compatibility":{"aria2":null,"gallery_dl":null}}';