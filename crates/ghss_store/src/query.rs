use crate::proto::{
    query_server::Query, IntervalAggregatesReply, IntervalAggregatesRequest, TotalAggregatesReply,
    TotalAggregatesRequest,
};
use crate::SQLiteStore;
use tonic::{Request, Response, Status};
use tracing::info;

#[tonic::async_trait]
impl Query for SQLiteStore {
    async fn get_total_aggregates(
        &self,
        request: Request<TotalAggregatesRequest>,
    ) -> Result<Response<TotalAggregatesReply>, Status> {
        info!("get_total_aggregates");
        let request = request.into_inner();
        let db = self.db_read(request.repository_id)?;
        Ok(Response::new(db.get_total_aggregates(
            request.table,
            request.columns,
            request.since,
            request.until,
            request.group_by,
        )?))
    }

    async fn get_interval_aggregates(
        &self,
        request: Request<IntervalAggregatesRequest>,
    ) -> Result<Response<IntervalAggregatesReply>, Status> {
        info!("get_interval_aggregates");
        let request = request.into_inner();
        let interval = request.interval();
        let db = self.db_read(request.repository_id)?;
        Ok(Response::new(db.get_interval_aggregates(
            request.table,
            request.columns,
            request.since,
            request.until,
            request.group_by,
            interval,
        )?))
    }
}
