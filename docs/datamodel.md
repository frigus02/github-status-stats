# Data model

## Build

- Measurement: `build`
- Tags:
  - `name`: `status.context` or `check_run.name`
  - `source`: `status` or `check_run`
- Fields:
  - `commit`: commit SHA
  - `successful`: `1` or `0`
  - `failed`: `1` or `0`
  - `duration_ms`: duration of build in milliseconds
- Timestamp: `status.created_at` or `check_run.started_at`

## Commit

- Measurement: `commit`
- Tags:
  - `build_name`: `status.context` or `check_run.name`
  - `build_source`: `status` or `check_run`
- Fields:
  - `commit`: commit SHA
  - `builds`: count of builds
  - `builds_successful`: count of successful builds
  - `builds_failed`: count of failed builds
- Timestamp: commit CommitDate

## Import

- Measurement: `import`
- Fields:
  - `points`: count of points
- Timestamp: import date

## Hook

- Measurement: `hook`
- Tags:
  - `type`: `status` or `check_run`
- Fields:
  - `commit`: commit SHA
- Timestamp: hook date
