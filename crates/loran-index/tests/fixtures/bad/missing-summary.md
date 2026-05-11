+++
name = "broken"
category = "test-only"
+++

Deliberately malformed page: `summary` is missing. The ingester must
surface this as an IngestError::Page.
