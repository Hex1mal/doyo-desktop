ALTER TABLE focus_sessions ADD COLUMN focus_workflow TEXT CHECK (focus_workflow IS NULL OR focus_workflow IN ('flowtime'));

CREATE INDEX IF NOT EXISTS idx_focus_sessions_workflow ON focus_sessions(focus_workflow);
