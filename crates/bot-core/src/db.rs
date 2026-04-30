use crawler::models::Competition;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::Row;
use sqlx::SqlitePool;

pub struct DbPool {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub enum Target {
    Group { group_id: i64 },
    Private { user_id: i64 },
}

impl Target {
    pub fn target_type(&self) -> &str {
        match self {
            Target::Group { .. } => "group",
            Target::Private { .. } => "private",
        }
    }

    pub fn target_id(&self) -> i64 {
        match self {
            Target::Group { group_id } => *group_id,
            Target::Private { user_id } => *user_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncScope {
    pub bili_parse: bool,
    pub competition: bool,
    pub welcome: bool,
}

impl DbPool {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        // Docker bind-mount creates a directory when the file doesn't exist.
        // Detect this early to give a clear error instead of SQLITE_CANTOPEN.
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.is_dir() {
                anyhow::bail!(
                    "Database path '{}' is a directory, not a file. \
                     Remove the directory (rm -rf {}) and ensure a regular file exists.",
                    path,
                    path
                );
            }
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    // ─── admins ───

    pub async fn add_admin(&self, user_id: i64) -> anyhow::Result<bool> {
        let rows = sqlx::query("INSERT OR IGNORE INTO admins (user_id) VALUES (?1)")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn remove_admin(&self, user_id: i64) -> anyhow::Result<bool> {
        let rows = sqlx::query("DELETE FROM admins WHERE user_id = ?1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn is_admin(&self, user_id: i64) -> anyhow::Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM admins WHERE user_id = ?1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0 > 0)
    }

    // ─── func_scopes ───

    pub async fn get_func_scope(
        &self,
        target_type: &str,
        target_id: i64,
    ) -> anyhow::Result<FuncScope> {
        sqlx::query("INSERT OR IGNORE INTO func_scopes (target_type, target_id) VALUES (?1, ?2)")
            .bind(target_type)
            .bind(target_id)
            .execute(&self.pool)
            .await?;

        let row: (i32, i32, i32) = sqlx::query_as(
            "SELECT bili_parse, competition, welcome FROM func_scopes WHERE target_type = ?1 AND target_id = ?2",
        )
        .bind(target_type)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(FuncScope {
            bili_parse: row.0 != 0,
            competition: row.1 != 0,
            welcome: row.2 != 0,
        })
    }

    pub async fn set_func_scope(
        &self,
        target_type: &str,
        target_id: i64,
        key: &str,
        value: bool,
    ) -> anyhow::Result<()> {
        let sql = match key {
            "bili_parse" => {
                "UPDATE func_scopes SET bili_parse  = ?1 WHERE target_type = ?2 AND target_id = ?3"
            }
            "competition" => {
                "UPDATE func_scopes SET competition = ?1 WHERE target_type = ?2 AND target_id = ?3"
            }
            "welcome" => {
                "UPDATE func_scopes SET welcome     = ?1 WHERE target_type = ?2 AND target_id = ?3"
            }
            _ => anyhow::bail!("invalid func_scope key: {key}"),
        };

        sqlx::query("INSERT OR IGNORE INTO func_scopes (target_type, target_id) VALUES (?1, ?2)")
            .bind(target_type)
            .bind(target_id)
            .execute(&self.pool)
            .await?;

        sqlx::query(sql)
            .bind(value as i32)
            .bind(target_type)
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_targets_with(&self, func: &str) -> anyhow::Result<Vec<Target>> {
        let sql = match func {
            "bili_parse" => "SELECT target_type, target_id FROM func_scopes WHERE bili_parse  = 1",
            "competition" => "SELECT target_type, target_id FROM func_scopes WHERE competition = 1",
            "welcome" => "SELECT target_type, target_id FROM func_scopes WHERE welcome     = 1",
            _ => anyhow::bail!("invalid func_scope key: {func}"),
        };

        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        let targets = rows
            .iter()
            .map(|row| {
                let tt: String = row.get(0);
                let id: i64 = row.get(1);
                match tt.as_str() {
                    "group" => Target::Group { group_id: id },
                    _ => Target::Private { user_id: id },
                }
            })
            .collect();
        Ok(targets)
    }

    pub async fn remove_func_scope(
        &self,
        target_type: &str,
        target_id: i64,
    ) -> anyhow::Result<bool> {
        let rows = sqlx::query("DELETE FROM func_scopes WHERE target_type = ?1 AND target_id = ?2")
            .bind(target_type)
            .bind(target_id)
            .execute(&self.pool)
            .await?;
        Ok(rows.rows_affected() > 0)
    }

    pub async fn get_all_func_scope_targets(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let rows = sqlx::query("SELECT DISTINCT target_type, target_id FROM func_scopes")
            .fetch_all(&self.pool)
            .await?;
        let targets = rows
            .iter()
            .map(|row| (row.get::<String, _>(0), row.get::<i64, _>(1)))
            .collect();
        Ok(targets)
    }

    // ─── group_welcome ───

    pub async fn get_welcome_message(&self, group_id: i64) -> anyhow::Result<Option<String>> {
        let result: Result<(String,), sqlx::Error> =
            sqlx::query_as("SELECT message FROM group_welcome WHERE group_id = ?1")
                .bind(group_id)
                .fetch_one(&self.pool)
                .await;

        match result {
            Ok(row) => Ok(Some(row.0)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn set_welcome_message(&self, group_id: i64, msg: &str) -> anyhow::Result<()> {
        sqlx::query("INSERT OR REPLACE INTO group_welcome (group_id, message) VALUES (?1, ?2)")
            .bind(group_id)
            .bind(msg)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ─── settings ───

    pub async fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        let result: Result<(String,), sqlx::Error> =
            sqlx::query_as("SELECT value FROM bot_settings WHERE key = ?1")
                .bind(key)
                .fetch_one(&self.pool)
                .await;

        match result {
            Ok(row) => Ok(Some(row.0)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        sqlx::query("INSERT OR REPLACE INTO bot_settings (key, value) VALUES (?1, ?2)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ─── competitions ───

    pub async fn upsert_competitions(&self, competitions: &[Competition]) -> anyhow::Result<()> {
        for c in competitions {
            sqlx::query(
                "INSERT INTO competitions (link, name, start_time, duration, platform, notified)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0)
                 ON CONFLICT(link) DO UPDATE SET
                     name       = excluded.name,
                     start_time = excluded.start_time,
                     duration   = excluded.duration,
                     platform   = excluded.platform
                 WHERE competitions.notified = 0",
            )
            .bind(&c.link)
            .bind(&c.name)
            .bind(c.start_time)
            .bind(c.duration)
            .bind(&c.platform)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_upcoming_competitions(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<Competition>> {
        let rows = sqlx::query(
            "SELECT link, name, start_time, duration, platform, notified
             FROM competitions
             WHERE start_time > CAST(strftime('%s','now') AS INTEGER)
             ORDER BY start_time ASC
             LIMIT ?1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let competitions = rows
            .iter()
            .map(|row| Competition {
                link: row.get(0),
                name: row.get(1),
                start_time: row.get(2),
                duration: row.get(3),
                platform: row.get(4),
                notified: row.get::<i32, _>(5) != 0,
            })
            .collect();
        Ok(competitions)
    }

    pub async fn get_pending_notifications(&self) -> anyhow::Result<Vec<Competition>> {
        let rows = sqlx::query(
            "SELECT link, name, start_time, duration, platform, notified
             FROM competitions
             WHERE start_time - 3600 <= CAST(strftime('%s','now') AS INTEGER)
               AND start_time > CAST(strftime('%s','now') AS INTEGER)
               AND notified = 0
             ORDER BY start_time ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let competitions = rows
            .iter()
            .map(|row| Competition {
                link: row.get(0),
                name: row.get(1),
                start_time: row.get(2),
                duration: row.get(3),
                platform: row.get(4),
                notified: row.get::<i32, _>(5) != 0,
            })
            .collect();
        Ok(competitions)
    }

    pub async fn mark_notified(&self, link: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE competitions SET notified = 1 WHERE link = ?1")
            .bind(link)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clean_expired(&self) -> anyhow::Result<()> {
        sqlx::query(
            "DELETE FROM competitions WHERE start_time + duration < CAST(strftime('%s','now') AS INTEGER) - 86400",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
