use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    // Service instances table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS service_instances (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            service_type TEXT NOT NULL,
            process_name TEXT,
            port INTEGER,
            exe TEXT,
            args TEXT,
            cwd TEXT,
            config_file TEXT,
            auto INTEGER DEFAULT 0,
            installed INTEGER DEFAULT 0,
            desired_state TEXT DEFAULT 'installed',
            actual_state TEXT DEFAULT 'unknown',
            pid INTEGER,
            error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // Sites table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sites (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            domain TEXT NOT NULL,
            port INTEGER NOT NULL,
            document_root TEXT NOT NULL,
            server_type TEXT NOT NULL,
            php_runtime_id TEXT,
            ssl INTEGER DEFAULT 0,
            status TEXT DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    add_column_if_missing(conn, "sites", "name", "TEXT NOT NULL DEFAULT ''")?;
    conn.execute(
        "UPDATE sites SET name = domain WHERE name IS NULL OR TRIM(name) = ''",
        [],
    )?;

    // Settings table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;

    // Operation logs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action TEXT NOT NULL,
            target_id TEXT,
            success INTEGER NOT NULL,
            error_code TEXT,
            message TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Runtimes table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runtimes (
            id TEXT PRIMARY KEY,
            runtime_type TEXT NOT NULL,
            version TEXT NOT NULL,
            install_path TEXT NOT NULL,
            entrypoint TEXT,
            config_template TEXT,
            installed INTEGER DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    )?;
    add_column_if_missing(conn, "runtimes", "config_template", "TEXT")?;

    // Databases table (composite key: same logical name may exist on MySQL 5.7 and 8.0)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS databases (
            name TEXT NOT NULL,
            user TEXT NOT NULL,
            password TEXT NOT NULL,
            engine TEXT DEFAULT 'mysql',
            size TEXT DEFAULT '',
            status TEXT DEFAULT 'active',
            mysql_service_id TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (name, mysql_service_id)
        )",
        [],
    )?;
    add_column_if_missing(conn, "databases", "mysql_service_id", "TEXT NOT NULL DEFAULT ''")?;
    migrate_databases_composite_pk(conn)?;

    // Software table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS software (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            service_id TEXT NOT NULL,
            download_url TEXT,
            install_dir TEXT,
            installed INTEGER DEFAULT 0,
            installable INTEGER DEFAULT 1,
            install_note TEXT,
            install_type TEXT DEFAULT 'local',
            install_path TEXT,
            status TEXT DEFAULT 'available',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    add_column_if_missing(conn, "software", "install_type", "TEXT DEFAULT 'local'")?;
    add_column_if_missing(conn, "software", "install_path", "TEXT")?;
    add_column_if_missing(conn, "software", "status", "TEXT DEFAULT 'available'")?;
    add_column_if_missing(conn, "software", "has_local", "INTEGER DEFAULT 0")?;
    add_column_if_missing(conn, "software", "has_bundled", "INTEGER DEFAULT 0")?;

    // Config files table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS config_files (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // Create indexes for common queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_service_instances_type ON service_instances(service_type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sites_domain ON sites(domain)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sites_name ON sites(name)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_created ON operation_logs(created_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_runtimes_type ON runtimes(runtime_type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_databases_name ON databases(name)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_software_category ON software(category)",
        [],
    )?;

    seed_default_data(conn)?;

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing?.eq_ignore_ascii_case(column) {
            return Ok(());
        }
    }

    conn.execute(
        &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
        [],
    )?;
    Ok(())
}

/// Rebuild `databases` so the primary key is `(name, mysql_service_id)`.
/// Older installs used `name` alone, which mixed MySQL 5.7 / 8.0 records.
fn migrate_databases_composite_pk(conn: &Connection) -> anyhow::Result<()> {
    let create_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'databases'",
            [],
            |row| row.get(0),
        )
        .ok();

    let Some(sql) = create_sql else {
        return Ok(());
    };
    let normalized = sql.to_lowercase().replace(' ', "");
    if normalized.contains("primarykey(name,mysql_service_id)") {
        return Ok(());
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS databases_v2 (
            name TEXT NOT NULL,
            user TEXT NOT NULL,
            password TEXT NOT NULL,
            engine TEXT DEFAULT 'mysql',
            size TEXT DEFAULT '',
            status TEXT DEFAULT 'active',
            mysql_service_id TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (name, mysql_service_id)
        )",
        [],
    )?;

    // Prefer currently installed/running MySQL for untagged legacy rows.
    let fallback_service: String = conn
        .query_row(
            "SELECT id FROM service_instances
             WHERE id IN ('mysql80', 'mysql57', 'mysql') AND installed = 1
             ORDER BY CASE WHEN pid IS NOT NULL THEN 0 ELSE 1 END,
                      CASE id WHEN 'mysql80' THEN 0 WHEN 'mysql57' THEN 1 ELSE 2 END
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "mysql80".to_string());

    conn.execute(
        "INSERT OR IGNORE INTO databases_v2
            (name, user, password, engine, size, status, mysql_service_id, created_at, updated_at)
         SELECT name, user, password, engine, size, status,
                CASE
                    WHEN mysql_service_id IS NULL OR TRIM(mysql_service_id) = '' THEN ?1
                    ELSE mysql_service_id
                END,
                created_at, updated_at
         FROM databases",
        rusqlite::params![fallback_service],
    )?;

    conn.execute("DROP TABLE databases", [])?;
    conn.execute("ALTER TABLE databases_v2 RENAME TO databases", [])?;
    Ok(())
}

/// Seed default data into the database if it is empty (first run).
pub fn seed_default_data(conn: &Connection) -> anyhow::Result<()> {
    // Only seed if service_instances is empty
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM service_instances",
        [],
        |row| row.get(0),
    )?;

    if count > 0 {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();

    // --- Default service instances ---
    let default_services = [
        // (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed)
        ("nginx", "Nginx", "nginx", "nginx.exe", 80i32, "", "", "", "", 1, 0),
        ("apache", "Apache2.4", "apache", "httpd.exe", 80i32, "", "", "", "", 0, 0),
        // Both MySQL versions share port 3306; only one is expected to run at a time.
        ("mysql57", "MySQL5.7", "mysql57", "mysqld.exe", 3306i32, "", "", "", "", 0, 0),
        ("mysql80", "MySQL8.0", "mysql80", "mysqld.exe", 3306i32, "", "", "", "", 1, 0),
        ("php73", "PHP7.3 CGI", "php", "php-cgi.exe", 9073i32, "", "", "", "", 1, 0),
        ("redis", "Redis", "redis", "redis-server.exe", 6379i32, "", "", "", "", 0, 0),
        ("pgsql", "PostgreSQL", "pgsql", "postgres.exe", 5432i32, "", "", "", "", 0, 0),
        ("minio", "MinIO", "minio", "minio.exe", 9000i32, "", "", "", "", 0, 0),
    ];

    for svc in &default_services {
        conn.execute(
            "INSERT INTO service_instances
             (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed, desired_state, actual_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'installed', 'unknown', ?12, ?13)",
            rusqlite::params![
                svc.0, svc.1, svc.2, svc.3, svc.4, svc.5, svc.6, svc.7, svc.8, svc.9, svc.10, &now, &now,
            ],
        )?;
    }

    // --- Default software entries ---
    let default_software = [
        // (id, name, category, service_id, download_url, install_dir, installed, installable, install_note)
        ("apache", "Apache2.4.39", "Web Servers", "apache", "", "", 0, 1, ""),
        ("nginx", "Nginx1.26.3", "Web Servers", "nginx", "https://nginx.org/download/nginx-1.26.3.zip", "", 0, 1, ""),
        ("mysql80", "MySQL8.0", "数据库", "mysql80", "", "", 0, 1, ""),
        ("mysql57", "MySQL5.7", "数据库", "mysql57", "", "", 0, 1, ""),
        ("redis", "Redis7.2.4", "缓存服务", "redis", "", "", 0, 1, ""),
        ("pgsql", "PostgreSQL16", "数据库", "pgsql", "", "", 0, 1, ""),
        ("minio", "MinIO", "对象存储", "minio", "", "", 0, 1, ""),
        ("mc", "MinIO Client", "对象存储", "minio", "", "", 0, 1, ""),
        ("php73", "php7.3.33nts", "运行环境", "php73", "", "", 0, 1, ""),
    ];

    for sw in &default_software {
        conn.execute(
            "INSERT INTO software
             (id, name, category, service_id, download_url, install_dir, installed, installable, install_note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                sw.0, sw.1, sw.2, sw.3, sw.4, sw.5, sw.6, sw.7, sw.8, &now, &now,
            ],
        )?;
    }

    // --- Default config files ---
    let default_config_files = [
        // (id, label, path)
        ("php.ini", "php.ini", ""),
        ("httpd.conf", "httpd.conf", ""),
        ("nginx.conf", "nginx.conf", ""),
        ("vhosts.conf", "vhosts.conf", ""),
        ("mysql.ini", "mysql.ini", ""),
        ("postgresql.conf", "postgresql.conf", ""),
        ("redis.conf", "redis.conf", ""),
        ("minio.env", "minio.env", ""),
        ("hosts", "hosts", "C:/Windows/System32/drivers/etc/hosts"),
    ];

    for cf in &default_config_files {
        conn.execute(
            "INSERT INTO config_files (id, label, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![cf.0, cf.1, cf.2, &now, &now],
        )?;
    }

    // --- Default settings ---
    let default_settings = [
        ("autostart", "false"),
        ("start_suite_on_launch", "false"),
        ("php_my_admin_url", "http://127.0.0.1/phpmyadmin"),
        ("port", "18113"),
        ("mysql_root_password", ""),
    ];

    for s in &default_settings {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![s.0, s.1],
        )?;
    }

    // --- Default site: localhost ---
    conn.execute(
        "INSERT INTO sites (id, name, domain, port, document_root, server_type, php_runtime_id, ssl, status, created_at, updated_at)
         VALUES ('localhost', 'localhost', 'localhost', 80, '', 'nginx', '', 0, 'active', ?1, ?2)",
        rusqlite::params![&now, &now],
    )?;

    Ok(())
}
