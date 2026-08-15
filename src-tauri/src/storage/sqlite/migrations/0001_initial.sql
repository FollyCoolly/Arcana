PRAGMA application_id = 0x41524341;

CREATE TABLE schema_migrations (
    version     INTEGER PRIMARY KEY CHECK (version > 0),
    name        TEXT NOT NULL,
    checksum    TEXT NOT NULL,
    applied_at  TEXT NOT NULL
) STRICT;

CREATE TABLE sync_state (
    singleton          INTEGER PRIMARY KEY CHECK (singleton = 1),
    repository_digest  TEXT,
    data_revision      INTEGER NOT NULL DEFAULT 0 CHECK (data_revision >= 0),
    exported_revision  INTEGER NOT NULL DEFAULT 0
        CHECK (exported_revision >= 0 AND exported_revision <= data_revision)
) STRICT;

INSERT INTO sync_state(singleton) VALUES (1);

CREATE TABLE packs (
    id             TEXT PRIMARY KEY,
    enabled        INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    manifest_json  TEXT NOT NULL
        CHECK (json_valid(manifest_json))
        CHECK (json_type(manifest_json) = 'object')
) STRICT;

CREATE TABLE pack_record_definitions (
    pack_id          TEXT NOT NULL,
    definition_id    TEXT NOT NULL,
    definition_json  TEXT NOT NULL
        CHECK (json_valid(definition_json))
        CHECK (json_type(definition_json) = 'object'),
    PRIMARY KEY (pack_id, definition_id),
    FOREIGN KEY (pack_id) REFERENCES packs(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_pack_record_definitions_definition
    ON pack_record_definitions(definition_id, pack_id);

CREATE TABLE pack_dimensions (
    pack_id         TEXT NOT NULL,
    dimension_id    TEXT NOT NULL,
    definition_json TEXT NOT NULL
        CHECK (json_valid(definition_json))
        CHECK (json_type(definition_json) = 'object'),
    PRIMARY KEY (pack_id, dimension_id),
    UNIQUE (dimension_id),
    FOREIGN KEY (pack_id) REFERENCES packs(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE status_dimension_selection (
    position      INTEGER PRIMARY KEY CHECK (position BETWEEN 0 AND 4),
    dimension_id TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE pack_achievements (
    pack_id          TEXT NOT NULL,
    achievement_id   TEXT NOT NULL,
    definition_json  TEXT NOT NULL
        CHECK (json_valid(definition_json))
        CHECK (json_type(definition_json) = 'object'),
    PRIMARY KEY (pack_id, achievement_id),
    UNIQUE (achievement_id),
    FOREIGN KEY (pack_id) REFERENCES packs(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE achievement_states (
    achievement_id TEXT PRIMARY KEY,
    status         TEXT NOT NULL CHECK (status IN ('tracked', 'achieved')),
    achieved_at    TEXT,
    CHECK (status = 'achieved' OR achieved_at IS NULL)
) STRICT;

CREATE INDEX idx_achievement_states_status
    ON achievement_states(status, achievement_id);

CREATE TABLE pack_skills (
    pack_id          TEXT NOT NULL,
    skill_id         TEXT NOT NULL,
    definition_json  TEXT NOT NULL
        CHECK (json_valid(definition_json))
        CHECK (json_type(definition_json) = 'object'),
    PRIMARY KEY (pack_id, skill_id),
    UNIQUE (skill_id),
    FOREIGN KEY (pack_id) REFERENCES packs(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE pack_assets (
    pack_id  TEXT NOT NULL,
    path     TEXT NOT NULL,
    content  BLOB NOT NULL,
    PRIMARY KEY (pack_id, path),
    FOREIGN KEY (pack_id) REFERENCES packs(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE records (
    definition_id  TEXT PRIMARY KEY,
    kind           TEXT NOT NULL
        CHECK (kind IN ('scalar', 'collection', 'event'))
) STRICT;

CREATE TABLE scalar_records (
    definition_id  TEXT PRIMARY KEY,
    value_json     TEXT NOT NULL
        CHECK (json_valid(value_json))
        CHECK (json_type(value_json) IN
            ('text', 'integer', 'real', 'true', 'false')),
    effective_at   TEXT,
    recorded_at    TEXT NOT NULL,
    FOREIGN KEY (definition_id) REFERENCES records(definition_id)
        ON DELETE CASCADE
) STRICT;

CREATE TABLE collection_items (
    definition_id  TEXT NOT NULL,
    item_id        TEXT NOT NULL,
    payload_json   TEXT NOT NULL
        CHECK (json_valid(payload_json))
        CHECK (json_type(payload_json) = 'object'),
    recorded_at    TEXT NOT NULL,
    PRIMARY KEY (definition_id, item_id),
    FOREIGN KEY (definition_id) REFERENCES records(definition_id)
        ON DELETE CASCADE
) STRICT;

CREATE TABLE event_entries (
    definition_id  TEXT NOT NULL,
    event_id        TEXT NOT NULL,
    occurred_at     TEXT NOT NULL,
    payload_json    TEXT NOT NULL
        CHECK (json_valid(payload_json))
        CHECK (json_type(payload_json) = 'object'),
    recorded_at     TEXT NOT NULL,
    PRIMARY KEY (definition_id, event_id),
    FOREIGN KEY (definition_id) REFERENCES records(definition_id)
        ON DELETE CASCADE
) STRICT;

CREATE INDEX idx_event_entries_time
    ON event_entries(definition_id, occurred_at, event_id);

CREATE TABLE missions (
    id            TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    description   TEXT,
    status        TEXT NOT NULL
        CHECK (status IN ('active', 'completed', 'archived')),
    progress      INTEGER CHECK (progress BETWEEN 0 AND 100),
    difficulty    TEXT CHECK (difficulty IN ('S', 'A', 'B', 'C', 'D')),
    deadline      TEXT,
    parent_id     TEXT,
    created_at    TEXT NOT NULL,
    completed_at  TEXT,
    CHECK (id <> parent_id),
    CHECK (status IN ('completed', 'archived') OR completed_at IS NULL),
    CHECK (status <> 'completed' OR progress IS NULL OR progress = 100),
    CHECK (completed_at IS NULL OR progress IS NULL OR progress = 100),
    FOREIGN KEY (parent_id) REFERENCES missions(id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE INDEX idx_missions_status_deadline
    ON missions(status, deadline, id);

CREATE INDEX idx_missions_parent
    ON missions(parent_id, id);

CREATE TABLE mission_suggestions (
    id                 TEXT PRIMARY KEY,
    title              TEXT NOT NULL,
    description        TEXT,
    difficulty         TEXT CHECK (difficulty IN ('S', 'A', 'B', 'C', 'D')),
    deadline           TEXT,
    parent_mission_id  TEXT,
    reason             TEXT,
    generated_at       TEXT NOT NULL,
    status             TEXT NOT NULL CHECK (status IN ('pending', 'rejected'))
) STRICT;

CREATE INDEX idx_mission_suggestions_status
    ON mission_suggestions(status, generated_at, id);

CREATE TABLE dashboard_mission_slots (
    slot        TEXT PRIMARY KEY
        CHECK (slot IN ('countdown', 'progress', 'hint_1', 'hint_2')),
    mission_id  TEXT NOT NULL,
    label       TEXT
) STRICT;

CREATE TABLE assistant_memories (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN (
        'focus', 'preference', 'constraint', 'habit',
        'summary', 'reminder', 'observation'
    )),
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

CREATE INDEX idx_assistant_memories_kind_updated
    ON assistant_memories(kind, updated_at, id);

CREATE TRIGGER records_kind_immutable
BEFORE UPDATE OF kind ON records
WHEN NEW.kind <> OLD.kind
BEGIN
    SELECT RAISE(ABORT, 'record kind is immutable');
END;

CREATE TRIGGER scalar_records_kind_guard
BEFORE INSERT ON scalar_records
WHEN (SELECT kind FROM records WHERE definition_id = NEW.definition_id)
    IS NOT 'scalar'
BEGIN
    SELECT RAISE(ABORT, 'scalar payload requires scalar record');
END;

CREATE TRIGGER collection_items_kind_guard
BEFORE INSERT ON collection_items
WHEN (SELECT kind FROM records WHERE definition_id = NEW.definition_id)
    IS NOT 'collection'
BEGIN
    SELECT RAISE(ABORT, 'collection item requires collection record');
END;

CREATE TRIGGER event_entries_kind_guard
BEFORE INSERT ON event_entries
WHEN (SELECT kind FROM records WHERE definition_id = NEW.definition_id)
    IS NOT 'event'
BEGIN
    SELECT RAISE(ABORT, 'event entry requires event record');
END;

CREATE TRIGGER scalar_records_parent_immutable
BEFORE UPDATE OF definition_id ON scalar_records
WHEN NEW.definition_id <> OLD.definition_id
BEGIN
    SELECT RAISE(ABORT, 'record payload parent is immutable');
END;

CREATE TRIGGER collection_items_parent_immutable
BEFORE UPDATE OF definition_id ON collection_items
WHEN NEW.definition_id <> OLD.definition_id
BEGIN
    SELECT RAISE(ABORT, 'record payload parent is immutable');
END;

CREATE TRIGGER event_entries_parent_immutable
BEFORE UPDATE OF definition_id ON event_entries
WHEN NEW.definition_id <> OLD.definition_id
BEGIN
    SELECT RAISE(ABORT, 'record payload parent is immutable');
END;
