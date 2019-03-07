use std::sync::atomic::{AtomicUsize, Ordering};

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
}
