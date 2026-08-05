use super::Migration;

const INITIAL_SCHEMA: &str = include_str!("v0001_create_audit_storage.sql");

pub(crate) fn all() -> Vec<Migration> {
    vec![Migration::sql(1, "create audit storage", INITIAL_SCHEMA)]
}
