use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Serialize;

#[derive(Serialize)]
pub struct StatsSerialized {
    pub data: Vec<(&'static str, usize)>,
}

pub struct Statistics {
    create_db: AtomicUsize,
    create_db_ok: AtomicUsize,
    create_db_err: AtomicUsize,
    exec: AtomicUsize,
    exec_ok: AtomicUsize,
    exec_err: AtomicUsize,
    query: AtomicUsize,
    query_ok: AtomicUsize,
    query_err: AtomicUsize,
    query_into: AtomicUsize,
    query_into_ok: AtomicUsize,
    query_into_err: AtomicUsize,
    create_statement: AtomicUsize,
    create_statement_ok: AtomicUsize,
    create_statement_err: AtomicUsize,
    exec_statement: AtomicUsize,
    exec_statement_ok: AtomicUsize,
    exec_statement_err: AtomicUsize,
    update_statement: AtomicUsize,
    update_statement_ok: AtomicUsize,
    update_statement_err: AtomicUsize,
    delete_statement: AtomicUsize,
    delete_statement_ok: AtomicUsize,
    delete_statement_err: AtomicUsize,
    query_statement: AtomicUsize,
    query_statement_ok: AtomicUsize,
    query_statement_err: AtomicUsize,
    query_statement_into: AtomicUsize,
    query_statement_into_ok: AtomicUsize,
    query_statement_into_err: AtomicUsize,
    copy: AtomicUsize,
    copy_ok: AtomicUsize,
    copy_err: AtomicUsize,
}

pub static STATISTICS: Statistics = Statistics {
    create_db: AtomicUsize::new(0),
    create_db_ok: AtomicUsize::new(0),
    create_db_err: AtomicUsize::new(0),
    exec: AtomicUsize::new(0),
    exec_ok: AtomicUsize::new(0),
    exec_err: AtomicUsize::new(0),
    query: AtomicUsize::new(0),
    query_ok: AtomicUsize::new(0),
    query_err: AtomicUsize::new(0),
    query_into: AtomicUsize::new(0),
    query_into_ok: AtomicUsize::new(0),
    query_into_err: AtomicUsize::new(0),
    create_statement: AtomicUsize::new(0),
    create_statement_ok: AtomicUsize::new(0),
    create_statement_err: AtomicUsize::new(0),
    exec_statement: AtomicUsize::new(0),
    exec_statement_ok: AtomicUsize::new(0),
    exec_statement_err: AtomicUsize::new(0),
    update_statement: AtomicUsize::new(0),
    update_statement_ok: AtomicUsize::new(0),
    update_statement_err: AtomicUsize::new(0),
    delete_statement: AtomicUsize::new(0),
    delete_statement_ok: AtomicUsize::new(0),
    delete_statement_err: AtomicUsize::new(0),
    query_statement: AtomicUsize::new(0),
    query_statement_ok: AtomicUsize::new(0),
    query_statement_err: AtomicUsize::new(0),
    query_statement_into: AtomicUsize::new(0),
    query_statement_into_ok: AtomicUsize::new(0),
    query_statement_into_err: AtomicUsize::new(0),
    copy: AtomicUsize::new(0),
    copy_ok: AtomicUsize::new(0),
    copy_err: AtomicUsize::new(0),
};

impl Statistics {
    pub fn create_db(&self) {
        STATISTICS.create_db.fetch_add(1, Ordering::Relaxed);
    }
    pub fn create_db_ok(&self) {
        STATISTICS.create_db_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn create_db_err(&self) {
        STATISTICS.create_db_err.fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec(&self) {
        STATISTICS.exec.fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec_ok(&self) {
        STATISTICS.exec_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec_err(&self) {
        STATISTICS.exec_err.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query(&self) {
        STATISTICS.query.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_ok(&self) {
        STATISTICS.query_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_err(&self) {
        STATISTICS.query_err.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_into(&self) {
        STATISTICS.query_into.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_into_ok(&self) {
        STATISTICS.query_into_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_into_err(&self) {
        STATISTICS.query_into_err.fetch_add(1, Ordering::Relaxed);
    }
    pub fn create_statement(&self) {
        STATISTICS.create_statement.fetch_add(1, Ordering::Relaxed);
    }
    pub fn create_statement_ok(&self) {
        STATISTICS
            .create_statement_ok
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn create_statement_err(&self) {
        STATISTICS
            .create_statement_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec_statement(&self) {
        STATISTICS.exec_statement.fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec_statement_ok(&self) {
        STATISTICS.exec_statement_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn exec_statement_err(&self) {
        STATISTICS
            .exec_statement_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn update_statement(&self) {
        STATISTICS.update_statement.fetch_add(1, Ordering::Relaxed);
    }
    pub fn update_statement_ok(&self) {
        STATISTICS
            .update_statement_ok
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn update_statement_err(&self) {
        STATISTICS
            .update_statement_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn delete_statement(&self) {
        STATISTICS.delete_statement.fetch_add(1, Ordering::Relaxed);
    }
    pub fn delete_statement_ok(&self) {
        STATISTICS
            .delete_statement_ok
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn delete_statement_err(&self) {
        STATISTICS
            .delete_statement_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement(&self) {
        STATISTICS.query_statement.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement_ok(&self) {
        STATISTICS
            .query_statement_ok
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement_err(&self) {
        STATISTICS
            .query_statement_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement_into(&self) {
        STATISTICS
            .query_statement_into
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement_into_ok(&self) {
        STATISTICS
            .query_statement_into_ok
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn query_statement_into_err(&self) {
        STATISTICS
            .query_statement_into_err
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn copy(&self) {
        STATISTICS.copy.fetch_add(1, Ordering::Relaxed);
    }
    pub fn copy_ok(&self) {
        STATISTICS.copy_ok.fetch_add(1, Ordering::Relaxed);
    }
    pub fn copy_err(&self) {
        STATISTICS.copy_err.fetch_add(1, Ordering::Relaxed);
    }
    pub fn values(&self) -> StatsSerialized {
        let mut stats: Vec<(&'static str, usize)> = Vec::new();
        stats.push((
            "CREATE_DB",
            self.create_db.load(Ordering::Relaxed),
        ));
        stats.push((
            "CREATE_DB OK",
            self.create_db_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "CREATE_DB ERR",
            self.create_db_err.load(Ordering::Relaxed),
        ));
        stats.push(("EXEC", self.exec.load(Ordering::Relaxed)));
        stats.push(("EXEC OK", self.exec_ok.load(Ordering::Relaxed)));
        stats.push((
            "EXEC ERR",
            self.exec_err.load(Ordering::Relaxed),
        ));

        stats.push(("QUERY", self.query.load(Ordering::Relaxed)));
        stats.push((
            "QUERY OK",
            self.query_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY ERR",
            self.query_err.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY.INTO",
            self.query_into.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY.INTO OK",
            self.query_into_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY.INTO ERR",
            self.query_into_err.load(Ordering::Relaxed),
        ));
        stats.push((
            "CREATE_STATEMENT",
            self.create_statement.load(Ordering::Relaxed),
        ));
        stats.push((
            "CREATE_STATEMENT OK",
            self.create_statement_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "CREATE_STATEMENT ERR",
            self.create_statement_err.load(Ordering::Relaxed),
        ));

        stats.push((
            "EXEC_STATEMENT",
            self.exec_statement.load(Ordering::Relaxed),
        ));
        stats.push((
            "EXEC_STATEMENT OK",
            self.exec_statement_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "EXEC_STATEMENT ERR",
            self.exec_statement_err.load(Ordering::Relaxed),
        ));

        stats.push((
            "UPDATE_STATEMENT",
            self.update_statement.load(Ordering::Relaxed),
        ));
        stats.push((
            "UPDATE_STATEMENT OK",
            self.update_statement_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "UPDATE_STATEMENT ERR",
            self.update_statement_err.load(Ordering::Relaxed),
        ));

        stats.push((
            "DELETE_STATEMENT",
            self.delete_statement.load(Ordering::Relaxed),
        ));
        stats.push((
            "DELETE_STATEMENT OK",
            self.delete_statement_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "DELETE_STATEMENT ERR",
            self.delete_statement_err.load(Ordering::Relaxed),
        ));

        stats.push((
            "QUERY_STATEMENT",
            self.query_statement.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY_STATEMENT OK",
            self.query_statement_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY_STATEMENT ERR",
            self.query_statement_err.load(Ordering::Relaxed),
        ));

        stats.push((
            "QUERY_STATEMENT.INTO",
            self.query_statement_into.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY_STATEMENT.INTO OK",
            self.query_statement_into_ok.load(Ordering::Relaxed),
        ));
        stats.push((
            "QUERY_STATEMENT.INTO ERR",
            self.query_statement_into_err.load(Ordering::Relaxed),
        ));

        stats.push(("COPY", self.copy.load(Ordering::Relaxed)));
        stats.push(("COPY OK", self.copy_ok.load(Ordering::Relaxed)));
        stats.push((
            "COPY ERR",
            self.copy_err.load(Ordering::Relaxed),
        ));

        StatsSerialized { data: stats }
    }

    pub fn serialize(&self) -> Result<String, serde_json::Error> {
        let stats = self.values();
        serde_json::to_string(&stats)
    }
}
