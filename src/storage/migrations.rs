use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

/// Database schema version
const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Run all necessary database migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    info!("Running database migrations");
    
    // Create schema_version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Get current schema version
    let current_version = get_schema_version(conn)?;
    info!("Current schema version: {}", current_version);
    
    // Apply migrations
    for version in (current_version + 1)..=CURRENT_SCHEMA_VERSION {
        info!("Applying migration to version {}", version);
        apply_migration(conn, version)?;
        update_schema_version(conn, version)?;
    }
    
    info!("Database migrations completed");
    Ok(())
}

/// Get current schema version
fn get_schema_version(conn: &Connection) -> Result<i32> {
    let mut stmt = conn.prepare("SELECT COALESCE(MAX(version), 0) FROM schema_version")?;
    let version: i32 = stmt.query_row([], |row| row.get(0))?;
    Ok(version)
}

/// Update schema version
fn update_schema_version(conn: &Connection, version: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
        [version, chrono::Utc::now().timestamp() as i32],
    )?;
    Ok(())
}

/// Apply specific migration
fn apply_migration(conn: &Connection, version: i32) -> Result<()> {
    match version {
        1 => apply_migration_v1(conn),
        _ => Err(anyhow::anyhow!("Unknown migration version: {}", version)),
    }
}

/// Migration v1: Initial schema
fn apply_migration_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration v1: Initial schema");
    
    // Jobs table
    conn.execute(
        "CREATE TABLE jobs (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            plan_yaml TEXT NOT NULL,
            user_prompt TEXT NOT NULL,
            settings_json TEXT
        )",
        [],
    )?;
    
    // Results table
    conn.execute(
        "CREATE TABLE results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            row_idx INTEGER NOT NULL,
            data_json TEXT NOT NULL,
            url TEXT NOT NULL,
            fetched_at INTEGER NOT NULL,
            hash TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES jobs (id) ON DELETE CASCADE,
            UNIQUE (job_id, hash)
        )",
        [],
    )?;
    
    // Logs table
    conn.execute(
        "CREATE TABLE logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id TEXT NOT NULL,
            ts INTEGER NOT NULL,
            stage TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES jobs (id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    // Cache table
    conn.execute(
        "CREATE TABLE cache (
            key TEXT PRIMARY KEY,
            value_blob BLOB NOT NULL,
            ttl INTEGER,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;
    
    // Create indexes for better performance
    conn.execute("CREATE INDEX idx_jobs_status ON jobs (status)", [])?;
    conn.execute("CREATE INDEX idx_jobs_created_at ON jobs (created_at)", [])?;
    conn.execute("CREATE INDEX idx_results_job_id ON results (job_id)", [])?;
    conn.execute("CREATE INDEX idx_results_hash ON results (hash)", [])?;
    conn.execute("CREATE INDEX idx_logs_job_id ON logs (job_id)", [])?;
    conn.execute("CREATE INDEX idx_logs_ts ON logs (ts)", [])?;
    conn.execute("CREATE INDEX idx_cache_ttl ON cache (ttl)", [])?;
    
    info!("Migration v1 completed successfully");
    Ok(())
}

// Future migrations can be added here
// Example:
// fn apply_migration_v2(conn: &Connection) -> Result<()> {
//     info!("Applying migration v2: Add new columns");
//     conn.execute("ALTER TABLE jobs ADD COLUMN priority INTEGER DEFAULT 0", [])?;
//     Ok(())
// }
