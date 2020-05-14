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
        unimplemented!()
    }

    async fn get_interval_aggregates(
        &self,
        request: Request<IntervalAggregatesRequest>,
    ) -> Result<Response<IntervalAggregatesReply>, Status> {
        info!("get_interval_aggregates");
        unimplemented!()
    }
}
