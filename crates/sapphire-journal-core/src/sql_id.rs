//! Store a [`GrainId`] in SQLite as its integer, as grain-id's own rusqlite
//! support (dropped in grain-id 0.17) did, so existing caches read the same.

use grain_id::GrainId;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};

/// A [`GrainId`] as an SQLite integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SqlId(pub GrainId);

impl ToSql for SqlId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer(self.0.to_u64() as i64)))
    }
}

impl FromSql for SqlId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let int = i64::column_result(value)?;
        Ok(Self(GrainId::from_u64_lossy(int as u64)))
    }
}

/// An optional [`GrainId`], such as a parent, as an SQLite integer or NULL.
pub(crate) fn opt(id: Option<GrainId>) -> Option<SqlId> {
    id.map(SqlId)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_round_trips_through_sqlite() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let id: GrainId = "012atvw".parse().unwrap();
        let (back, int): (SqlId, i64) = conn
            .query_row("SELECT ?1, ?1", [SqlId(id)], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(back.0, id);
        assert_eq!(int as u64, id.to_u64(), "stored as its integer");
    }
}
