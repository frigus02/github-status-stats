# Data model

## Build

- Measurement: `build`
- Tags:
  - `name`: `status.context` or `check_run.name`
  - `source`: `status` or `check_run`
- Fields:
  - `commit`: commit SHA
  - `successful`: `true` or `false`
  - `duration_ms`
- Timestamp: `status.created_at` or `check_run.started_at`

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
