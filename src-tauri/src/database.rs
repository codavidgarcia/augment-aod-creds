use sqlx::{sqlite::SqlitePool, Row};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceRecord {
    pub id: Uuid,
    pub amount: u32,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: Uuid,
    pub start_balance: u32,
    pub end_balance: u32,
    pub usage_amount: u32,
    pub duration_minutes: u32,
    pub timestamp: DateTime<Utc>,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> AppResult<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Database(sqlx::Error::Configuration("Could not find data directory".into())))?;

        let db_dir = data_dir.join("orb-credit-monitor");
        tokio::fs::create_dir_all(&db_dir).await?;

        let db_path = db_dir.join("data.db");
        let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&database_url).await?;
        
        let database = Self { pool };
        database.run_migrations().await?;
        
        Ok(database)
    }
    
    async fn run_migrations(&self) -> AppResult<()> {
        // Create balance_records table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS balance_records (
                id TEXT PRIMARY KEY,
                amount INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'scraper'
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create usage_records table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_records (
                id TEXT PRIMARY KEY,
                start_balance INTEGER NOT NULL,
                end_balance INTEGER NOT NULL,
                usage_amount INTEGER NOT NULL,
                duration_minutes INTEGER NOT NULL,
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_balance_timestamp ON balance_records(timestamp)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_records(timestamp)")
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn insert_balance_record(&self, amount: u32) -> AppResult<BalanceRecord> {
        let record = BalanceRecord {
            id: Uuid::new_v4(),
            amount,
            timestamp: Utc::now(),
            source: "scraper".to_string(),
        };
        
        sqlx::query(
            "INSERT INTO balance_records (id, amount, timestamp, source) VALUES (?, ?, ?, ?)"
        )
        .bind(record.id.to_string())
        .bind(record.amount as i64)
        .bind(record.timestamp.to_rfc3339())
        .bind(&record.source)
        .execute(&self.pool)
        .await?;
        
        // Calculate usage if we have a previous record
        if let Ok(Some(previous)) = self.get_previous_balance_record().await {
            if previous.amount > amount {
                let usage_amount = previous.amount - amount;
                let duration = record.timestamp.signed_duration_since(previous.timestamp);
                let duration_minutes = duration.num_minutes().max(1) as u32;
                
                self.insert_usage_record(previous.amount, amount, usage_amount, duration_minutes).await?;
            }
        }
        
        Ok(record)
    }
    
    pub async fn insert_usage_record(
        &self,
        start_balance: u32,
        end_balance: u32,
        usage_amount: u32,
        duration_minutes: u32,
    ) -> AppResult<UsageRecord> {
        let record = UsageRecord {
            id: Uuid::new_v4(),
            start_balance,
            end_balance,
            usage_amount,
            duration_minutes,
            timestamp: Utc::now(),
        };
        
        sqlx::query(
            "INSERT INTO usage_records (id, start_balance, end_balance, usage_amount, duration_minutes, timestamp) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(record.id.to_string())
        .bind(record.start_balance as i64)
        .bind(record.end_balance as i64)
        .bind(record.usage_amount as i64)
        .bind(record.duration_minutes as i64)
        .bind(record.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(record)
    }
    
    pub async fn get_latest_balance(&self) -> AppResult<Option<BalanceRecord>> {
        let row = sqlx::query(
            "SELECT id, amount, timestamp, source FROM balance_records ORDER BY timestamp DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            Ok(Some(BalanceRecord {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?,
                amount: row.get::<i64, _>("amount") as u32,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?
                    .with_timezone(&Utc),
                source: row.get("source"),
            }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_previous_balance_record(&self) -> AppResult<Option<BalanceRecord>> {
        let row = sqlx::query(
            "SELECT id, amount, timestamp, source FROM balance_records ORDER BY timestamp DESC LIMIT 1 OFFSET 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            Ok(Some(BalanceRecord {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?,
                amount: row.get::<i64, _>("amount") as u32,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?
                    .with_timezone(&Utc),
                source: row.get("source"),
            }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_balance_history(&self, hours: u32) -> AppResult<Vec<BalanceRecord>> {
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        
        let rows = sqlx::query(
            "SELECT id, amount, timestamp, source FROM balance_records WHERE timestamp >= ? ORDER BY timestamp ASC"
        )
        .bind(since.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;
        
        let mut records = Vec::new();
        for row in rows {
            records.push(BalanceRecord {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?,
                amount: row.get::<i64, _>("amount") as u32,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?
                    .with_timezone(&Utc),
                source: row.get("source"),
            });
        }
        
        Ok(records)
    }
    
    pub async fn get_usage_history(&self, hours: u32) -> AppResult<Vec<UsageRecord>> {
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        
        let rows = sqlx::query(
            "SELECT id, start_balance, end_balance, usage_amount, duration_minutes, timestamp FROM usage_records WHERE timestamp >= ? ORDER BY timestamp ASC"
        )
        .bind(since.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;
        
        let mut records = Vec::new();
        for row in rows {
            records.push(UsageRecord {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?,
                start_balance: row.get::<i64, _>("start_balance") as u32,
                end_balance: row.get::<i64, _>("end_balance") as u32,
                usage_amount: row.get::<i64, _>("usage_amount") as u32,
                duration_minutes: row.get::<i64, _>("duration_minutes") as u32,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?
                    .with_timezone(&Utc),
            });
        }
        
        Ok(records)
    }
    
    pub async fn cleanup_old_records(&self, retention_days: u32) -> AppResult<()> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        
        sqlx::query("DELETE FROM balance_records WHERE timestamp < ?")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM usage_records WHERE timestamp < ?")
            .bind(cutoff.to_rfc3339())
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}
